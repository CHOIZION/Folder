import type { Category, LibrarySnapshot, ScannedItem } from "./types";
import { getAllItems } from "./library.ts";

export type HeartMode = "basic" | "handsfree" | "jarvis";
export type WarmupLevel = "none" | "light" | "full";

export interface SessionStep {
  id: string;
  item: ScannedItem;
  categoryName: string;
  durationMinutes: number;
  reason: string;
}

export interface SessionPlan {
  id: string;
  name: string;
  description: string;
  requestedMinutes: number;
  warmup: WarmupLevel;
  createdAt: string;
  steps: SessionStep[];
}

export type VoiceIntent =
  | { type: "mode"; mode: HeartMode }
  | { type: "category"; categoryName: string }
  | { type: "scroll"; direction: "up" | "down" }
  | { type: "random" }
  | { type: "next" }
  | { type: "previous" }
  | { type: "open" }
  | { type: "favorite" }
  | { type: "stop" }
  | { type: "session"; minutes: number; warmup: WarmupLevel }
  | { type: "unknown"; transcript: string; suggestion?: string };

const CATEGORY_ALIASES: Record<string, string[]> = {
  만화: ["만화", "코믹스", "comics"],
  이미지: ["이미지", "그림", "사진", "images", "photos"],
  게임: ["게임", "games"],
  렌파이: ["렌파이", "렘파이", "renpy"],
  쯔꾸르: ["쯔꾸르", "쯔구르", "rpgm"],
  영상: ["영상", "비디오", "애니메이션", "videos", "movies"],
  문서: ["문서", "소설", "전자책", "documents", "books"],
};

export function parseVoiceIntent(rawTranscript: string): VoiceIntent {
  const transcript = cleanSpeech(rawTranscript);
  const compact = compactSpeech(transcript);
  if (!compact) return { type: "unknown", transcript };

  const requestedMinutes = extractRequestedMinutes(transcript);
  const jarvisWakeWord = /(자비스|쟈비스|자 비스|쟈 비스)/.test(transcript);
  if (
    requestedMinutes !== null &&
    jarvisWakeWord &&
    /(세팅|셋팅|코스|course)/.test(compact)
  ) {
    const requested = Math.min(180, Math.max(5, requestedMinutes));
    const warmup: WarmupLevel = /(도입|워밍업|워밍|준비)/.test(compact)
      ? "full"
      : /(가볍게|짧게|간단히)/.test(compact)
        ? "light"
        : "none";
    return { type: "session", minutes: requested, warmup };
  }

  if (/(기본모드|일반모드)/.test(compact))
    return { type: "mode", mode: "basic" };
  if (/(핸즈프리|핸드프리|헨즈프리|핸즈후리)/.test(compact)) {
    return { type: "mode", mode: "handsfree" };
  }
  if (/(자비스|쟈비스)/.test(compact)) return { type: "mode", mode: "jarvis" };

  const command = removeCommandFillers(compact);
  if (/^(랜덤|랜덤선택|아무거나|무작위)$/.test(command))
    return { type: "random" };
  if (/^(다음|다음거|다음것|넘겨|넘기기)$/.test(command))
    return { type: "next" };
  if (/^(이전|이전거|이전것|뒤로|전으로)$/.test(command))
    return { type: "previous" };
  if (/^(실행|열어|열기|재생|시작|틀어|틀어줘)$/.test(command))
    return { type: "open" };
  if (/^(즐겨찾기|좋아요|좋아|찜|찜하기)$/.test(command))
    return { type: "favorite" };
  if (/^(중지|정지|멈춰|세션중지)$/.test(command)) return { type: "stop" };
  if (/^(종료|끝|닫아|꺼|꺼줘)$/.test(command)) return { type: "next" };
  if (/^(아래|내려|내려가|아래스크롤)$/.test(command)) {
    return { type: "scroll", direction: "down" };
  }
  if (/^(위|올려|올라가|위스크롤)$/.test(command)) {
    return { type: "scroll", direction: "up" };
  }

  for (const [categoryName, aliases] of Object.entries(CATEGORY_ALIASES)) {
    if (aliases.some((alias) => categoryCommandMatches(command, alias))) {
      return { type: "category", categoryName };
    }
  }

  return {
    type: "unknown",
    transcript: rawTranscript,
    suggestion: suggestCommand(command),
  };
}

function cleanSpeech(value: string): string {
  return value
    .toLowerCase()
    .normalize("NFKC")
    .replace(/[.,!?。？！,，:;~…'"“”‘’()[\]{}]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function compactSpeech(value: string): string {
  return value.replace(/\s+/g, "");
}

function removeCommandFillers(value: string): string {
  return value
    .replace(/^(하트야|하트|자비스|쟈비스)/, "")
    .replace(/(해줘|해주세요|부탁해|부탁해요|좀|으로|모드)$/g, "");
}

function categoryCommandMatches(command: string, alias: string): boolean {
  const target = compactSpeech(cleanSpeech(alias));
  const candidate = command.replace(
    /(탭|카테고리|목록|으로|보여줘|열어줘)$/g,
    "",
  );
  return (
    candidate === target ||
    (target.length >= 3 &&
      candidate.length >= 3 &&
      levenshtein(candidate, target) <= 1)
  );
}

function extractRequestedMinutes(transcript: string): number | null {
  const digitMatch = transcript.match(/(\d{1,3})\s*분/);
  if (digitMatch) return Number(digitMatch[1]);

  const compact = compactSpeech(transcript);
  const minuteIndex = compact.indexOf("분");
  if (minuteIndex < 0) return null;
  const beforeMinute = compact
    .slice(0, minuteIndex)
    .replace(/^.*?(자비스|쟈비스)/, "")
    .replace(/(세팅|셋팅|코스|도입|워밍업|워밍|가볍게)/g, "");
  return parseKoreanNumber(beforeMinute);
}

function parseKoreanNumber(value: string): number | null {
  const native: Record<string, number> = {
    한: 1,
    하나: 1,
    두: 2,
    둘: 2,
    세: 3,
    셋: 3,
    네: 4,
    넷: 4,
    다섯: 5,
    여섯: 6,
    일곱: 7,
    여덟: 8,
    아홉: 9,
    열: 10,
    스물: 20,
    서른: 30,
    마흔: 40,
    쉰: 50,
    예순: 60,
    일흔: 70,
    여든: 80,
    아흔: 90,
  };
  if (native[value] !== undefined) return native[value];

  const digits: Record<string, number> = {
    영: 0,
    공: 0,
    일: 1,
    이: 2,
    삼: 3,
    사: 4,
    오: 5,
    육: 6,
    칠: 7,
    팔: 8,
    구: 9,
  };
  let total = 0;
  let current = 0;
  let recognized = false;
  for (const character of value) {
    if (digits[character] !== undefined) {
      current = digits[character];
      recognized = true;
    } else if (character === "십") {
      total += (current || 1) * 10;
      current = 0;
      recognized = true;
    } else if (character === "백") {
      total += (current || 1) * 100;
      current = 0;
      recognized = true;
    }
  }
  return recognized ? total + current : null;
}

function suggestCommand(command: string): string | undefined {
  const candidates = [
    ...Object.keys(CATEGORY_ALIASES),
    "아래",
    "위",
    "랜덤",
    "다음",
    "이전",
    "실행",
    "즐겨찾기",
    "중지",
    "종료",
  ];
  const nearest = candidates
    .map((candidate) => ({
      candidate,
      distance: levenshtein(command, candidate),
    }))
    .sort((a, b) => a.distance - b.distance)[0];
  return nearest && nearest.distance <= 2 ? nearest.candidate : undefined;
}

function levenshtein(left: string, right: string): number {
  const rows = Array.from({ length: left.length + 1 }, () =>
    Array<number>(right.length + 1).fill(0),
  );
  for (let i = 0; i <= left.length; i += 1) rows[i][0] = i;
  for (let j = 0; j <= right.length; j += 1) rows[0][j] = j;
  for (let i = 1; i <= left.length; i += 1) {
    for (let j = 1; j <= right.length; j += 1) {
      rows[i][j] = Math.min(
        rows[i - 1][j] + 1,
        rows[i][j - 1] + 1,
        rows[i - 1][j - 1] + (left[i - 1] === right[j - 1] ? 0 : 1),
      );
    }
  }
  return rows[left.length][right.length];
}

export function findCategoryIdByVoice(
  categories: Category[],
  requestedName: string,
): string | null {
  const targets = CATEGORY_ALIASES[requestedName] ?? [requestedName];
  const normalizedTargets = targets.map(normalize);

  function visit(nodes: Category[]): string | null {
    for (const node of nodes) {
      const name = normalize(node.name);
      if (
        normalizedTargets.some(
          (target) => name.includes(target) || target.includes(name),
        )
      ) {
        return node.id;
      }
      const child = visit(node.children);
      if (child) return child;
    }
    return null;
  }

  return visit(categories);
}

export function buildSessionPlans(
  snapshot: LibrarySnapshot,
  requestedMinutes: number,
  warmup: WarmupLevel,
): SessionPlan[] {
  return [
    buildSessionPlan(snapshot, requestedMinutes, warmup, "balanced", 0),
    buildSessionPlan(snapshot, requestedMinutes, warmup, "game", 1),
    buildSessionPlan(snapshot, requestedMinutes, warmup, "visual", 2),
  ].filter((plan) => plan.steps.length > 0);
}

function buildSessionPlan(
  snapshot: LibrarySnapshot,
  requestedMinutes: number,
  warmup: WarmupLevel,
  style: "balanced" | "game" | "visual",
  seed: number,
): SessionPlan {
  const allItems = getAllItems(
    snapshot.categories,
    snapshot.unclassifiedItems,
  ).filter((item) => !item.missing);
  const categoryByItem = buildCategoryLookup(snapshot.categories);
  const selectedPaths = new Set<string>();
  const steps: SessionStep[] = [];

  const phases = createPhases(requestedMinutes, warmup, style);
  for (const phase of phases) {
    const candidates = allItems
      .filter((item) => !selectedPaths.has(item.path))
      .map((item) => ({
        item,
        category: categoryByItem.get(item.path) ?? "미분류",
        score: scoreItem(
          item,
          categoryByItem.get(item.path) ?? "",
          phase.kinds,
          phase.intent,
          style,
          seed,
        ),
      }))
      .filter((candidate) => candidate.score > -500)
      .sort((a, b) => b.score - a.score);

    const chosen = weightedChoice(
      candidates.slice(0, Math.min(12, candidates.length)),
    );
    if (!chosen) continue;
    selectedPaths.add(chosen.item.path);
    steps.push({
      id: `${steps.length}-${chosen.item.path}`,
      item: chosen.item,
      categoryName: chosen.category,
      durationMinutes: phase.minutes,
      reason: describeReason(chosen.item, chosen.category, phase.intent),
    });
  }

  if (steps.length === 0 && allItems.length > 0) {
    const item = allItems[Math.floor(Math.random() * allItems.length)];
    steps.push({
      id: `0-${item.path}`,
      item,
      categoryName: categoryByItem.get(item.path) ?? "미분류",
      durationMinutes: requestedMinutes,
      reason: "사용 가능한 전체 라이브러리에서 선택",
    });
  }

  return {
    id: `${style}-${Date.now()}-${seed}`,
    name:
      style === "balanced"
        ? "균형 코스"
        : style === "game"
          ? "게임 중심 코스"
          : "영상·이미지 중심 코스",
    description:
      style === "balanced"
        ? "게임·이미지·영상을 고르게 구성"
        : style === "game"
          ? "게임 시간을 길게, 나머지는 전환 구간으로 구성"
          : "짧은 영상과 이미지 비중을 높여 빠르게 전환",
    requestedMinutes,
    warmup,
    createdAt: new Date().toISOString(),
    steps,
  };
}

function createPhases(
  total: number,
  warmup: WarmupLevel,
  style: "balanced" | "game" | "visual",
) {
  const phases: Array<{
    minutes: number;
    kinds: string[];
    intent: "warmup" | "main" | "finish";
  }> = [];
  const targetCount = Math.max(2, Math.min(12, Math.round(total / 11)));
  const weights = Array.from({ length: targetCount }, (_, index) => {
    if (warmup !== "none" && index === 0) return 0.8;
    if (index === targetCount - 1) return 0.9;
    return style === "game" && index % 2 === 0 ? 1.35 : 1;
  });
  const weightTotal = weights.reduce((sum, weight) => sum + weight, 0);
  let allocated = 0;

  for (let index = 0; index < targetCount; index += 1) {
    const last = index === targetCount - 1;
    const minutes = last
      ? total - allocated
      : Math.max(4, Math.round((total * weights[index]) / weightTotal));
    allocated += minutes;
    const intent =
      index === 0 && warmup !== "none" ? "warmup" : last ? "finish" : "main";
    const kinds =
      intent === "warmup"
        ? ["image", "video"]
        : style === "game"
          ? index % 2 === 0
            ? ["game"]
            : ["image", "video"]
          : style === "visual"
            ? ["video", "image"]
            : index % 3 === 0
              ? ["game"]
              : ["image", "video"];
    phases.push({ minutes, kinds, intent });
  }

  return phases;
}

function scoreItem(
  item: ScannedItem,
  category: string,
  kinds: string[],
  intent: "warmup" | "main" | "finish",
  style: "balanced" | "game" | "visual",
  seed: number,
): number {
  if (!kinds.includes(item.itemType)) return -1000;
  let score = Math.random() * 18;
  score += (item.rating ?? 2.5) * 14;
  score += item.favorite ? 22 : 0;
  score += Math.min(item.openCount, 8) * 2;
  score -= recencyPenalty(item.lastOpenedAt);

  if (intent === "warmup") {
    if (item.itemType === "image") score += 25;
    if (/이미지|사진|만화|images|photos|comics/i.test(category)) score += 18;
  } else if (intent === "finish") {
    if (item.itemType === "video") score += 30;
    if (/영상|비디오|애니메이션|videos|movies/i.test(category)) score += 24;
  } else if (item.itemType === "game") {
    score += 20;
  }
  if (style === "game" && item.itemType === "game") score += 24;
  if (style === "visual" && item.itemType !== "game") score += 22;
  score += ((hashText(item.path) + seed * 97) % 31) / 3;
  return score;
}

function hashText(value: string): number {
  let hash = 0;
  for (let index = 0; index < value.length; index += 1) {
    hash = ((hash << 5) - hash + value.charCodeAt(index)) | 0;
  }
  return Math.abs(hash);
}

function recencyPenalty(lastOpenedAt: string | null): number {
  if (!lastOpenedAt) return -10;
  const ageHours = (Date.now() - new Date(lastOpenedAt).getTime()) / 3_600_000;
  if (ageHours < 24) return 35;
  if (ageHours < 72) return 18;
  if (ageHours < 168) return 8;
  return 0;
}

function weightedChoice<T extends { score: number }>(items: T[]): T | null {
  if (items.length === 0) return null;
  const minimum = Math.min(...items.map((item) => item.score));
  const weights = items.map((item) => Math.max(1, item.score - minimum + 1));
  const total = weights.reduce((sum, weight) => sum + weight, 0);
  let cursor = Math.random() * total;
  for (let index = 0; index < items.length; index += 1) {
    cursor -= weights[index];
    if (cursor <= 0) return items[index];
  }
  return items[0];
}

function buildCategoryLookup(categories: Category[]): Map<string, string> {
  const lookup = new Map<string, string>();
  function visit(nodes: Category[], ancestors: string[]) {
    for (const node of nodes) {
      const label = [...ancestors, node.name].join(" › ");
      for (const item of node.items) lookup.set(item.path, label);
      visit(node.children, [...ancestors, node.name]);
    }
  }
  visit(categories, []);
  return lookup;
}

function describeReason(
  item: ScannedItem,
  category: string,
  intent: "warmup" | "main" | "finish",
): string {
  const phase =
    intent === "warmup" ? "도입" : intent === "finish" ? "마무리" : "메인";
  const signals = [
    item.rating ? `평점 ${item.rating}` : "",
    item.favorite ? "즐겨찾기" : "",
    item.tags.slice(0, 2).join(", "),
  ].filter(Boolean);
  return `${phase} 구간 · ${category}${signals.length ? ` · ${signals.join(" · ")}` : ""}`;
}

function normalize(value: string): string {
  return value.toLowerCase().replace(/[\s'’_-]+/g, "");
}
