import type { ScannedItem } from "./types";
import { normalizeSearchText, type TagCatalogEntry } from "./tags.ts";

export interface SearchDocument {
  text: string;
  tags: string[];
}

export type SearchIndex = ReadonlyMap<string, SearchDocument>;
export type TagSuggestionMatch = "exact" | "prefix" | "contains" | "related";

export interface TagSuggestion extends TagCatalogEntry {
  match: TagSuggestionMatch;
}

const SEARCH_SEPARATOR = /[,，;\n]+/;
const ACTIVE_SEPARATOR = /[,，;\n]/g;

export function parseSearchTerms(query: string): string[] {
  const terms = query
    .split(SEARCH_SEPARATOR)
    .map(normalizeSearchText)
    .filter(Boolean);

  return [...new Set(terms)];
}

export function createSearchIndex(items: readonly ScannedItem[]): SearchIndex {
  return new Map(items.map((item) => [item.path, createSearchDocument(item)]));
}

export function filterItems(
  items: readonly ScannedItem[],
  query: string,
  index: SearchIndex = createSearchIndex(items),
): ScannedItem[] {
  const terms = parseSearchTerms(query);
  if (terms.length === 0) return [...items];

  return items.filter((item) => {
    const document = index.get(item.path) ?? createSearchDocument(item);
    return terms.every((term) => matchesTerm(document, term));
  });
}

export function getActiveSearchFragment(query: string): string {
  let start = 0;
  for (const match of query.matchAll(ACTIVE_SEPARATOR)) {
    start = (match.index ?? 0) + match[0].length;
  }
  return query.slice(start).trim();
}

export function applyTagSuggestion(query: string, tag: string): string {
  const separators = [...query.matchAll(ACTIVE_SEPARATOR)];
  const lastSeparator = separators.at(-1);
  const prefix = lastSeparator
    ? query.slice(0, (lastSeparator.index ?? 0) + lastSeparator[0].length)
    : "";
  const completedPrefix = prefix.trim().replace(/[,，;]+$/, "");
  return completedPrefix ? `${completedPrefix}, ${tag}, ` : `${tag}, `;
}

export function getTagSuggestions(
  catalog: readonly TagCatalogEntry[],
  query: string,
  limit = 8,
): TagSuggestion[] {
  const fragment = normalizeSearchText(getActiveSearchFragment(query));
  if (!fragment || limit <= 0) return [];

  const selected = new Set(parseSearchTerms(query));
  selected.delete(fragment);

  return catalog
    .filter((entry) => !selected.has(entry.normalized))
    .map((entry) => {
      const result = scoreSuggestion(entry, fragment);
      return result ? { entry, ...result } : null;
    })
    .filter(
      (
        result,
      ): result is {
        entry: TagCatalogEntry;
        match: TagSuggestionMatch;
        score: number;
      } => result !== null,
    )
    .sort(
      (left, right) =>
        left.score - right.score ||
        right.entry.count - left.entry.count ||
        left.entry.tag.localeCompare(right.entry.tag, "ko-KR", {
          sensitivity: "base",
        }),
    )
    .slice(0, limit)
    .map(({ entry, match }) => ({ ...entry, match }));
}

function createSearchDocument(item: ScannedItem): SearchDocument {
  const tags = item.tags.map(normalizeSearchText);
  return {
    tags,
    text: [item.title, item.path, item.itemType, item.notes, ...tags]
      .map(normalizeSearchText)
      .join("\n"),
  };
}

function matchesTerm(document: SearchDocument, term: string): boolean {
  if (hasTagPrefix(term)) {
    const separator = term.indexOf(":");
    const requestedKind = term.slice(0, separator).trim();
    const requestedLabel = term.slice(separator + 1).trim();

    return document.tags.some((tag) => {
      const tagSeparator = tag.indexOf(":");
      if (tagSeparator < 0) return false;
      const tagKind = tag.slice(0, tagSeparator).trim();
      const tagLabel = tag.slice(tagSeparator + 1).trim();
      return tagKind === requestedKind && tagLabel.includes(requestedLabel);
    });
  }
  return document.text.includes(term);
}

function hasTagPrefix(term: string): boolean {
  return /^[^\s:/\\]+\s*:(?![\\/])/.test(term);
}

function scoreSuggestion(
  entry: TagCatalogEntry,
  fragment: string,
): { match: TagSuggestionMatch; score: number } | null {
  if (entry.normalized === fragment) {
    return { match: "exact", score: 0 };
  }
  if (entry.normalized.startsWith(fragment)) {
    return { match: "prefix", score: 10 };
  }
  if (entry.normalizedLabel.startsWith(fragment)) {
    return { match: "prefix", score: 12 };
  }

  const fragmentLabel = normalizeSearchText(
    fragment.split(":").at(-1) ?? fragment,
  );
  const requestedKind = fragment.includes(":")
    ? fragment.slice(0, fragment.indexOf(":"))
    : null;
  const kindMatches = requestedKind === null || requestedKind === entry.kind;

  if (kindMatches && entry.normalizedLabel.startsWith(fragmentLabel)) {
    return { match: "prefix", score: 14 };
  }
  if (
    entry.normalized.includes(fragment) ||
    entry.normalizedLabel.includes(fragmentLabel)
  ) {
    return { match: "contains", score: 20 };
  }

  const comparison =
    requestedKind && kindMatches ? entry.normalizedLabel : entry.normalized;
  const needle = requestedKind && kindMatches ? fragmentLabel : fragment;
  const distance = levenshteinDistance(comparison, needle);
  const threshold = needle.length <= 4 ? 1 : needle.length <= 9 ? 2 : 3;
  return distance <= threshold
    ? { match: "related", score: 30 + distance }
    : null;
}

function levenshteinDistance(left: string, right: string): number {
  if (left === right) return 0;
  if (!left) return right.length;
  if (!right) return left.length;

  let previous = Array.from({ length: right.length + 1 }, (_, index) => index);
  for (let leftIndex = 1; leftIndex <= left.length; leftIndex += 1) {
    const current = [leftIndex];
    for (let rightIndex = 1; rightIndex <= right.length; rightIndex += 1) {
      const substitutionCost =
        left[leftIndex - 1] === right[rightIndex - 1] ? 0 : 1;
      current[rightIndex] = Math.min(
        current[rightIndex - 1] + 1,
        previous[rightIndex] + 1,
        previous[rightIndex - 1] + substitutionCost,
      );
    }
    previous = current;
  }
  return previous[right.length];
}
