const SEARCH_SEPARATOR = /[,，;\n]+/;
const ACTIVE_SEPARATOR = /[,，;\n]/g;

export function normalizeSearchText(value) {
  return String(value ?? "")
    .normalize("NFKC")
    .trim()
    .toLocaleLowerCase();
}

export function parseSearchTerms(query) {
  return [
    ...new Set(
      String(query ?? "")
        .split(SEARCH_SEPARATOR)
        .map(normalizeSearchText)
        .filter(Boolean),
    ),
  ];
}

export function createSearchIndex(items) {
  return new Map(
    items.map((item) => {
      const tags = (item.tags ?? []).map(normalizeSearchText);
      const text = [item.title, item.path, item.itemType, item.notes, ...tags]
        .map(normalizeSearchText)
        .join("\n");
      return [item.path, { tags, text }];
    }),
  );
}

export function filterItemsByQuery(
  items,
  query,
  index = createSearchIndex(items),
) {
  const terms = parseSearchTerms(query);
  if (terms.length === 0) return items;

  return items.filter((item) => {
    const document =
      index.get(item.path) ?? createSearchIndex([item]).get(item.path);
    return (
      document &&
      terms.every((term) => matchesTerm(document.text, document.tags, term))
    );
  });
}

export function buildTagCatalog(items) {
  const catalog = new Map();
  for (const item of items) {
    for (const rawTag of item.tags ?? []) {
      const tag = rawTag.trim();
      const normalized = normalizeSearchText(tag);
      if (!normalized) continue;

      const existing = catalog.get(normalized);
      if (existing) {
        existing.count += 1;
        continue;
      }

      const separator = tag.indexOf(":");
      const label = (separator >= 0 ? tag.slice(separator + 1) : tag).trim();
      catalog.set(normalized, {
        tag,
        normalized,
        label,
        normalizedLabel: normalizeSearchText(label),
        kind: tagKind(normalized),
        count: 1,
      });
    }
  }
  return [...catalog.values()];
}

export function getTagSuggestions(catalog, query, limit = 8) {
  const fragment = normalizeSearchText(activeFragment(query));
  if (!fragment || limit <= 0) return [];

  const selected = new Set(parseSearchTerms(query));
  selected.delete(fragment);

  return catalog
    .filter((entry) => !selected.has(entry.normalized))
    .map((entry) => {
      const result = suggestionScore(entry, fragment);
      return result ? { ...entry, ...result } : null;
    })
    .filter(Boolean)
    .sort(
      (left, right) =>
        left.score - right.score ||
        right.count - left.count ||
        left.tag.localeCompare(right.tag, "ko-KR", { sensitivity: "base" }),
    )
    .slice(0, limit);
}

export function applyTagSuggestion(query, tag) {
  const separators = [...String(query ?? "").matchAll(ACTIVE_SEPARATOR)];
  const lastSeparator = separators.at(-1);
  const prefix = lastSeparator
    ? query.slice(0, (lastSeparator.index ?? 0) + lastSeparator[0].length)
    : "";
  const completedPrefix = prefix.trim().replace(/[,，;]+$/, "");
  return completedPrefix ? `${completedPrefix}, ${tag}, ` : `${tag}, `;
}

function matchesTerm(text, tags, term) {
  if (!/^[^\s:/\\]+\s*:(?![\\/])/.test(term)) return text.includes(term);

  const separator = term.indexOf(":");
  const requestedKind = term.slice(0, separator).trim();
  const requestedLabel = term.slice(separator + 1).trim();
  return tags.some((tag) => {
    const tagSeparator = tag.indexOf(":");
    return (
      tagSeparator >= 0 &&
      tag.slice(0, tagSeparator).trim() === requestedKind &&
      tag
        .slice(tagSeparator + 1)
        .trim()
        .includes(requestedLabel)
    );
  });
}

function activeFragment(query) {
  let start = 0;
  for (const match of String(query ?? "").matchAll(ACTIVE_SEPARATOR)) {
    start = (match.index ?? 0) + match[0].length;
  }
  return query.slice(start).trim();
}

function tagKind(normalizedTag) {
  if (normalizedTag.startsWith("topic:")) return "topic";
  if (normalizedTag.startsWith("genre:")) return "genre";
  return "etc";
}

function suggestionScore(entry, fragment) {
  if (entry.normalized === fragment) return { match: "정확", score: 0 };
  if (entry.normalized.startsWith(fragment))
    return { match: "자동완성", score: 10 };
  if (entry.normalizedLabel.startsWith(fragment))
    return { match: "자동완성", score: 12 };

  const fragmentLabel = normalizeSearchText(fragment.split(":").at(-1));
  const requestedKind = fragment.includes(":")
    ? fragment.slice(0, fragment.indexOf(":"))
    : null;
  const kindMatches = requestedKind === null || requestedKind === entry.kind;

  if (kindMatches && entry.normalizedLabel.startsWith(fragmentLabel)) {
    return { match: "자동완성", score: 14 };
  }
  if (
    entry.normalized.includes(fragment) ||
    entry.normalizedLabel.includes(fragmentLabel)
  ) {
    return { match: "포함", score: 20 };
  }

  const comparison =
    requestedKind && kindMatches ? entry.normalizedLabel : entry.normalized;
  const needle = requestedKind && kindMatches ? fragmentLabel : fragment;
  const distance = levenshteinDistance(comparison, needle);
  const threshold = needle.length <= 4 ? 1 : needle.length <= 9 ? 2 : 3;
  return distance <= threshold ? { match: "연관", score: 30 + distance } : null;
}

function levenshteinDistance(left, right) {
  if (left === right) return 0;
  if (!left) return right.length;
  if (!right) return left.length;

  let previous = Array.from({ length: right.length + 1 }, (_, index) => index);
  for (let leftIndex = 1; leftIndex <= left.length; leftIndex += 1) {
    const current = [leftIndex];
    for (let rightIndex = 1; rightIndex <= right.length; rightIndex += 1) {
      const cost = left[leftIndex - 1] === right[rightIndex - 1] ? 0 : 1;
      current[rightIndex] = Math.min(
        current[rightIndex - 1] + 1,
        previous[rightIndex] + 1,
        previous[rightIndex - 1] + cost,
      );
    }
    previous = current;
  }
  return previous[right.length];
}
