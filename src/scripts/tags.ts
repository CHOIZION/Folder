import type { ScannedItem } from "./types";

export type TagKind = "genre" | "topic" | "etc";

export interface TagCatalogEntry {
  tag: string;
  normalized: string;
  kind: TagKind;
  label: string;
  normalizedLabel: string;
  count: number;
}

const TAG_PREFIXES: Readonly<Record<Exclude<TagKind, "etc">, string>> = {
  genre: "genre:",
  topic: "topic:",
};

export function normalizeSearchText(value: string): string {
  return value.normalize("NFKC").trim().toLocaleLowerCase();
}

export function getTagKind(tag: string): TagKind {
  const normalized = normalizeSearchText(tag);
  if (normalized.startsWith(TAG_PREFIXES.genre)) return "genre";
  if (normalized.startsWith(TAG_PREFIXES.topic)) return "topic";
  return "etc";
}

export function getTagLabel(tag: string): string {
  const colon = tag.indexOf(":");
  return (colon >= 0 ? tag.slice(colon + 1) : tag).trim();
}

export function buildTagCatalog(
  items: ReadonlyArray<Pick<ScannedItem, "tags">>,
): TagCatalogEntry[] {
  const entries = new Map<string, TagCatalogEntry>();

  for (const item of items) {
    for (const rawTag of item.tags) {
      const tag = rawTag.trim();
      const normalized = normalizeSearchText(tag);
      if (!normalized) continue;

      const existing = entries.get(normalized);
      if (existing) {
        existing.count += 1;
        continue;
      }

      const label = getTagLabel(tag);
      entries.set(normalized, {
        tag,
        normalized,
        kind: getTagKind(tag),
        label,
        normalizedLabel: normalizeSearchText(label),
        count: 1,
      });
    }
  }

  return [...entries.values()].sort((left, right) =>
    left.tag.localeCompare(right.tag, "ko-KR", {
      numeric: true,
      sensitivity: "base",
    }),
  );
}
