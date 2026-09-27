import {
  buildSessionPlans,
  findCategoryIdByVoice,
  parseVoiceIntent,
  type HeartMode,
  type SessionPlan,
  type VoiceIntent,
  type WarmupLevel,
} from "./assistant";
import type { LibraryController } from "./library-controller.svelte";
import { stopLaunchedItem } from "./heart-api";

type SpeechRecognitionLike = {
  lang: string;
  continuous: boolean;
  interimResults: boolean;
  maxAlternatives: number;
  onresult: ((event: any) => void) | null;
  onerror: ((event: any) => void) | null;
  onend: (() => void) | null;
  start(): void;
  stop(): void;
};

export class AssistantController {
  open = $state(false);
  mode = $state<HeartMode>("basic");
  listening = $state(false);
  supported = $state(false);
  transcript = $state("");
  response = $state("");
  plans = $state<SessionPlan[]>([]);
  selectedPlanId = $state<string | null>(null);
  running = $state(false);
  stepIndex = $state(0);
  remainingSeconds = $state(0);

  private recognition: SpeechRecognitionLike | null = null;
  private timer: ReturnType<typeof setInterval> | null = null;

  constructor(private readonly library: LibraryController) {}

  start(): void {
    const SpeechRecognition =
      (window as any).SpeechRecognition ??
      (window as any).webkitSpeechRecognition;
    if (!SpeechRecognition) return;

    this.recognition = new SpeechRecognition() as SpeechRecognitionLike;
    this.recognition.lang = "ko-KR";
    this.recognition.continuous = true;
    this.recognition.interimResults = false;
    this.recognition.maxAlternatives = 5;
    this.recognition.onresult = (event) => this.handleRecognitionResult(event);
    this.recognition.onerror = (event) => {
      this.listening = false;
      this.response = `음성 인식 오류: ${event.error ?? "알 수 없음"}`;
    };
    this.recognition.onend = () => {
      if (!this.listening) return;
      try {
        this.recognition?.start();
      } catch {
        /* browser owns restart timing */
      }
    };
    this.supported = true;
  }

  dispose(): void {
    this.listening = false;
    this.recognition?.stop();
    this.clearTimer();
  }

  toggleListening = (): void => {
    if (!this.recognition || !this.supported) return;
    if (this.listening) {
      this.listening = false;
      this.recognition.stop();
      this.response = "음성 인식을 중지했습니다.";
      return;
    }

    this.listening = true;
    this.open = true;
    try {
      this.recognition.start();
      this.response = "듣고 있습니다.";
    } catch {
      this.response = "음성 인식을 다시 시작하지 못했습니다.";
    }
  };

  setMode = (mode: HeartMode): void => {
    this.mode = mode;
    this.open = true;
    if (mode === "basic" && this.listening) this.toggleListening();
  };

  selectPlan = (planId: string): void => {
    if (!this.running) this.selectedPlanId = planId;
  };

  startSession = async (): Promise<void> => {
    const plan = this.selectedPlan();
    if (!plan?.steps.length) return;
    this.running = true;
    this.stepIndex = Math.min(this.stepIndex, plan.steps.length - 1);
    await this.startCurrentStep();
  };

  nextStep = async (): Promise<void> => {
    const plan = this.selectedPlan();
    if (!plan) return;
    await this.stopLaunchedItem();
    if (this.stepIndex >= plan.steps.length - 1) {
      this.stopSession();
      this.response = "코스가 끝났습니다.";
      return;
    }
    this.stepIndex += 1;
    await this.startCurrentStep();
  };

  previousStep = async (): Promise<void> => {
    if (!this.selectedPlan()) return;
    await this.stopLaunchedItem();
    this.stepIndex = Math.max(0, this.stepIndex - 1);
    await this.startCurrentStep();
  };

  stopSession = (): void => {
    this.running = false;
    this.remainingSeconds = 0;
    this.clearTimer();
    void this.stopLaunchedItem();
  };

  private handleRecognitionResult(event: any): void {
    const result = event.results[event.results.length - 1];
    const alternatives = Array.from(
      { length: result?.length ?? 0 },
      (_, index) => String(result[index]?.transcript ?? "").trim(),
    ).filter(Boolean);
    const matched = alternatives
      .map((transcript) => ({
        transcript,
        intent: parseVoiceIntent(transcript),
      }))
      .find((candidate) => candidate.intent.type !== "unknown");
    const transcript = matched?.transcript ?? alternatives[0] ?? "";
    if (!transcript) return;
    this.transcript = transcript;
    void this.execute(matched?.intent ?? parseVoiceIntent(transcript));
  }

  private async execute(intent: VoiceIntent): Promise<void> {
    this.open = true;
    switch (intent.type) {
      case "mode":
        this.setMode(intent.mode);
        this.response = `${modeLabel(intent.mode)} 모드로 전환했습니다.`;
        break;
      case "category":
        this.selectCategoryByVoice(intent.categoryName);
        break;
      case "scroll": {
        const region = document.querySelector<HTMLElement>(
          "[data-heart-scroll-region]",
        );
        region?.scrollBy({
          top: intent.direction === "down" ? 520 : -520,
          behavior: "smooth",
        });
        this.response =
          intent.direction === "down"
            ? "아래로 이동했습니다."
            : "위로 이동했습니다.";
        break;
      }
      case "random":
        this.library.selectRandomItem();
        this.response = this.library.selectedItem
          ? `${this.library.selectedItem.title}을 선택했습니다.`
          : "랜덤 작품을 선택했습니다.";
        break;
      case "open":
        if (this.library.selectedItem) {
          const title = this.library.selectedItem.title;
          await this.library.openItem(this.library.selectedItem);
          this.response = `${title}을 실행했습니다.`;
        } else {
          this.response = "먼저 작품을 선택해 주세요.";
        }
        break;
      case "next":
        if (this.running) await this.nextStep();
        else this.moveSelection(1);
        break;
      case "previous":
        if (this.running) await this.previousStep();
        else this.moveSelection(-1);
        break;
      case "favorite":
        await this.toggleFavorite();
        break;
      case "stop":
        this.stopSession();
        this.response = "세션을 종료했습니다.";
        break;
      case "session":
        this.buildPlans(intent.minutes, intent.warmup);
        break;
      case "unknown":
        this.response = intent.suggestion
          ? `“${intent.suggestion}” 명령인가요? 다시 한 번 짧게 말해 주세요.`
          : `명령을 이해하지 못했습니다: ${intent.transcript}`;
        break;
    }
  }

  private selectCategoryByVoice(categoryName: string): void {
    const snapshot = this.library.snapshot;
    if (!snapshot) return;
    const id = findCategoryIdByVoice(snapshot.categories, categoryName);
    if (!id) {
      this.response = `${categoryName} 카테고리를 찾지 못했습니다.`;
      return;
    }
    this.library.selectCategory(id);
    this.response = `${categoryName} 탭으로 이동했습니다.`;
  }

  private moveSelection(direction: number): void {
    const item = this.library.moveSelection(direction);
    if (item) this.response = `${item.title}을 선택했습니다.`;
  }

  private async toggleFavorite(): Promise<void> {
    const item = this.library.selectedItem;
    if (!item) {
      this.response = "선택된 작품이 없습니다.";
      return;
    }
    const wasFavorite = item.favorite;
    await this.library.saveMetadata({
      path: item.path,
      favorite: !wasFavorite,
      rating: item.rating,
      notes: item.notes,
      customThumbnailPath: item.customThumbnailPath,
      tags: item.tags,
    });
    this.response = wasFavorite
      ? "즐겨찾기에서 제거했습니다."
      : "즐겨찾기에 추가했습니다.";
  }

  private buildPlans(minutes: number, warmup: WarmupLevel): void {
    if (!this.library.snapshot) return;
    this.mode = "jarvis";
    this.plans = buildSessionPlans(this.library.snapshot, minutes, warmup);
    this.selectedPlanId = this.plans[0]?.id ?? null;
    this.stepIndex = 0;
    this.running = false;
    this.response = `${minutes}분${warmup !== "none" ? " 도입 포함" : ""} 코스를 구성했습니다.`;
  }

  private selectedPlan(): SessionPlan | null {
    return (
      this.plans.find((plan) => plan.id === this.selectedPlanId) ??
      this.plans[0] ??
      null
    );
  }

  private async startCurrentStep(): Promise<void> {
    const plan = this.selectedPlan();
    if (!plan || !this.running) return;
    const step = plan.steps[this.stepIndex];
    this.library.selectItem(step.item);
    this.remainingSeconds = step.durationMinutes * 60;
    this.clearTimer();
    await this.library.openItem(step.item);
    this.response = `${step.item.title} · ${step.durationMinutes}분 구간을 시작했습니다.`;
    this.timer = setInterval(() => {
      this.remainingSeconds -= 1;
      if (this.remainingSeconds <= 0) void this.nextStep();
    }, 1000);
  }

  private async stopLaunchedItem(): Promise<void> {
    try {
      await stopLaunchedItem();
    } catch (error) {
      this.response = `현재 프로그램 종료 실패: ${error instanceof Error ? error.message : String(error)}`;
    }
  }

  private clearTimer(): void {
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
  }
}

function modeLabel(mode: HeartMode): string {
  if (mode === "jarvis") return "자비스";
  if (mode === "handsfree") return "핸즈프리";
  return "기본";
}
