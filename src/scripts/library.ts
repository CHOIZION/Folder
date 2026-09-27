import type { Category, LibrarySnapshot, ScannedItem } from "./types";

export const UNCLASSIFIED_CATEGORY_ID = "__unclassified__";
export const FAVORITES_CATEGORY_ID = "__favorites__";
export const RECENT_CATEGORY_ID = "__recent__";
export const USER_CATEGORY_PREFIX = "__user__:";

export interface CategoryItemSection {
  id: string;
  title: string;
  breadcrumb: string;
  depth: number;
  isRoot: boolean;
  items: ScannedItem[];
}

export type SortMode = "abc-asc" | "abc-desc" | "ko-asc" | "ko-desc";

const collators: Record<"en-US" | "ko-KR", Intl.Collator> = {
  "en-US": new Intl.Collator("en-US", { numeric: true, sensitivity: "base" }),
  "ko-KR": new Intl.Collator("ko-KR", { numeric: true, sensitivity: "base" }),
};

export function getAllItems(
  categories: Category[],
  unclassifiedItems: ScannedItem[],
): ScannedItem[] {
  const items = [
    ...collectItemsFromCategories(categories),
    ...unclassifiedItems,
  ];
  return [...new Map(items.map((item) => [item.path, item])).values()];
}

export function getItemsForCategory(
  snapshot: LibrarySnapshot,
  categoryId: string | null,
  cachedAllItems?: ScannedItem[],
): ScannedItem[] {
  const allItems =
    cachedAllItems ??
    getAllItems(snapshot.categories, snapshot.unclassifiedItems);

  if (categoryId === null) return allItems;
  if (categoryId === UNCLASSIFIED_CATEGORY_ID) {
    return allItems.filter(needsClassification);
  }
  if (categoryId === FAVORITES_CATEGORY_ID) {
    return allItems.filter((item) => item.favorite);
  }
  if (categoryId === RECENT_CATEGORY_ID) {
    return allItems
      .filter((item) => item.lastOpenedAt)
      .sort((a, b) =>
        String(b.lastOpenedAt).localeCompare(String(a.lastOpenedAt)),
      );
  }
  if (categoryId.startsWith(USER_CATEGORY_PREFIX)) {
    const id = Number(categoryId.slice(USER_CATEGORY_PREFIX.length));
    const category = snapshot.userCategories.find((entry) => entry.id === id);
    const paths = new Set(category?.itemPaths ?? []);
    return allItems.filter((item) => paths.has(item.path));
  }

  const category = findCategory(snapshot.categories, categoryId);
  return category ? collectItemsFromCategory(category) : [];
}

export function needsClassification(item: ScannedItem): boolean {
  return item.rating === null || item.tags.length === 0;
}

export function findCategory(
  categories: Category[],
  categoryId: string,
): Category | null {
  for (const category of categories) {
    if (category.id === categoryId) return category;
    const child = findCategory(category.children, categoryId);
    if (child) return child;
  }
  return null;
}

export function collectItemsFromCategories(
  categories: Category[],
): ScannedItem[] {
  return categories.flatMap(collectItemsFromCategory);
}

export function collectItemsFromCategory(category: Category): ScannedItem[] {
  return [
    ...category.items,
    ...category.children.flatMap(collectItemsFromCategory),
  ];
}

export function findItem(
  snapshot: LibrarySnapshot,
  path: string,
): ScannedItem | null {
  return (
    getAllItems(snapshot.categories, snapshot.unclassifiedItems).find(
      (item) => item.path === path,
    ) ?? null
  );
}

export function filterItemsByRating(
  items: ScannedItem[],
  rating: number | null,
): ScannedItem[] {
  if (rating === null) return items;
  return items.filter((item) => item.rating === rating);
}

export function sortItems(items: ScannedItem[], mode: SortMode): ScannedItem[] {
  const locale = mode.startsWith("ko") ? "ko-KR" : "en-US";
  const direction = mode.endsWith("desc") ? -1 : 1;
  const collator = collators[locale];

  return [...items].sort(
    (a, b) => collator.compare(a.title, b.title) * direction,
  );
}

export function getCategoryItemSections(
  category: Category,
): CategoryItemSection[] {
  const sections: CategoryItemSection[] = [];

  function visit(
    current: Category,
    ancestors: string[],
    depth: number,
    isRoot: boolean,
  ): void {
    const breadcrumbParts = isRoot ? [] : [...ancestors, current.name];

    sections.push({
      id: current.id,
      title: current.name,
      breadcrumb: breadcrumbParts.join(" › "),
      depth,
      isRoot,
      items: current.items,
    });

    const nextAncestors = isRoot ? [] : [...ancestors, current.name];

    for (const child of current.children) {
      visit(child, nextAncestors, depth + 1, false);
    }
  }

  visit(category, [], 0, true);
  return sections;
}
