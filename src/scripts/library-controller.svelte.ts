import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";

import {
  createUserCategory,
  deleteUserCategory,
  getDatabasePath,
  launchItem,
  launchMediaFile,
  loadLibrary,
  recordItemOpen,
  scanFolder,
  setItemUserCategories,
  updateItemMetadata,
  watchLibrary,
} from "./heart-api";
import {
  FAVORITES_CATEGORY_ID,
  RECENT_CATEGORY_ID,
  UNCLASSIFIED_CATEGORY_ID,
  USER_CATEGORY_PREFIX,
  filterItemsByRating,
  findCategory,
  getAllItems,
  getCategoryItemSections,
  getItemsForCategory,
  needsClassification,
  sortItems,
  type SortMode,
} from "./library";
import { createSearchIndex, filterItems } from "./search";
import { buildTagCatalog } from "./tags";
import type { LibrarySnapshot, MetadataUpdate, ScannedItem } from "./types";

export type ActiveView = "library" | "memo";

export class LibraryController {
  rootPath = $state("");
  snapshot = $state<LibrarySnapshot | null>(null);
  selectedCategoryId = $state<string | null>(null);
  selectedItemPath = $state<string | null>(null);
  searchText = $state("");
  selectedRating = $state<number | null>(null);
  sortMode = $state<SortMode>("ko-asc");
  loading = $state(false);
  saving = $state(false);
  errorMessage = $state("");
  databasePath = $state("");
  statisticsOpen = $state(false);
  detailPanelOpen = $state(false);
  activeView = $state<ActiveView>("library");

  allItems = $derived.by(() =>
    this.snapshot
      ? getAllItems(this.snapshot.categories, this.snapshot.unclassifiedItems)
      : [],
  );
  searchIndex = $derived(createSearchIndex(this.allItems));
  tagCatalog = $derived(buildTagCatalog(this.allItems));
  selectedItem = $derived.by(
    () =>
      this.allItems.find((item) => item.path === this.selectedItemPath) ?? null,
  );
  visibleItems = $derived.by(() => {
    if (!this.snapshot) return [];
    const categoryItems = getItemsForCategory(
      this.snapshot,
      this.selectedCategoryId,
      this.allItems,
    );
    return this.filterAndSort(categoryItems);
  });
  visibleSections = $derived.by(() => {
    if (
      !this.snapshot ||
      !this.selectedCategoryId ||
      this.selectedCategoryId.startsWith("__")
    ) {
      return null;
    }
    const category = findCategory(
      this.snapshot.categories,
      this.selectedCategoryId,
    );
    if (!category?.children.length) return null;
    return getCategoryItemSections(category)
      .map((section) => ({
        ...section,
        items: this.filterAndSort(section.items),
      }))
      .filter((section) => section.items.length > 0);
  });
  currentTitle = $derived.by(() => this.resolveCurrentTitle());
  counts = $derived.by(() => {
    const ratingCounts = [0, 0, 0, 0, 0, 0];
    let favorites = 0;
    let recent = 0;
    let unclassified = 0;
    for (const item of this.allItems) {
      if (item.favorite) favorites += 1;
      if (item.lastOpenedAt) recent += 1;
      if (needsClassification(item)) unclassified += 1;
      if (item.rating !== null && item.rating >= 0 && item.rating <= 5) {
        ratingCounts[item.rating] += 1;
      }
    }
    return { favorites, recent, unclassified, ratingCounts };
  });
  pulse = $derived.by(() => {
    const items: ScannedItem[] = [];
    let favorites = 0;
    let recent = 0;
    let oldest: ScannedItem | null = null;
    let latest: ScannedItem | null = null;
    let oldestAt = Number.POSITIVE_INFINITY;
    let latestAt = 0;
    for (const item of this.allItems) {
      if (item.missing) continue;
      items.push(item);
      if (item.favorite) favorites += 1;
      const openedAt = item.lastOpenedAt ? Date.parse(item.lastOpenedAt) : 0;
      if (!Number.isFinite(openedAt) || openedAt <= 0) continue;
      recent += 1;
      if (openedAt < oldestAt) {
        oldestAt = openedAt;
        oldest = item;
      }
      if (openedAt > latestAt) {
        latestAt = openedAt;
        latest = item;
      }
    }
    return { items, favorites, recent, oldest, latest };
  });

  initialize = async (): Promise<void> => {
    this.loading = true;
    this.errorMessage = "";
    try {
      this.databasePath = await getDatabasePath();
      this.rootPath = localStorage.getItem("heart.libraryRoot") ?? "";
      if (!this.rootPath) return;
      try {
        this.snapshot = await loadLibrary(this.rootPath);
      } catch {
        this.snapshot = await scanFolder(this.rootPath);
      }
      await watchLibrary(this.snapshot.rootPath);
    } catch (error) {
      this.setError(error);
    } finally {
      this.loading = false;
    }
  };

  reloadFromWatcher = async (rootPath: string): Promise<void> => {
    if (!this.snapshot || this.snapshot.rootPath !== rootPath) return;
    this.backgroundReloadQueued = true;
    if (this.backgroundReloading) return;

    this.backgroundReloading = true;
    try {
      while (this.backgroundReloadQueued) {
        this.backgroundReloadQueued = false;
        const snapshot = await loadLibrary(rootPath);
        if (this.rootPath !== rootPath) break;
        this.snapshot = snapshot;
        if (this.selectedItemPath && !this.selectedItem) {
          this.selectedItemPath = null;
          this.detailPanelOpen = false;
        }
      }
    } catch (error) {
      this.setError(error);
    } finally {
      this.backgroundReloading = false;
    }
  };

  chooseRoot = async (path: string): Promise<void> => {
    const candidate = path.trim().replace(/^"(.*)"$/, "$1");
    if (!candidate || this.loading) return;
    this.loading = true;
    this.errorMessage = "";
    try {
      const snapshot = await scanFolder(candidate);
      this.rootPath = snapshot.rootPath;
      this.snapshot = snapshot;
      this.selectedCategoryId = null;
      this.selectedItemPath = null;
      this.detailPanelOpen = false;
      localStorage.setItem("heart.libraryRoot", this.rootPath);
      await watchLibrary(this.rootPath);
    } catch (error) {
      this.setError(error);
    } finally {
      this.loading = false;
    }
  };

  rescan = async (): Promise<void> => {
    if (!this.rootPath || this.loading) return;
    this.loading = true;
    this.errorMessage = "";
    try {
      this.snapshot = await scanFolder(
        this.snapshot?.rootPath ?? this.rootPath,
      );
      if (this.selectedItemPath && !this.selectedItem)
        this.selectedItemPath = null;
    } catch (error) {
      this.setError(error);
    } finally {
      this.loading = false;
    }
  };

  saveMetadata = async (update: MetadataUpdate): Promise<void> => {
    if (!this.snapshot) return;
    this.saving = true;
    this.errorMessage = "";
    try {
      this.snapshot = await updateItemMetadata(this.snapshot.rootPath, update);
      this.selectedItemPath = update.path;
    } catch (error) {
      this.setError(error);
    } finally {
      this.saving = false;
    }
  };

  saveCategories = async (
    item: ScannedItem,
    categoryIds: number[],
  ): Promise<void> => {
    if (!this.snapshot) return;
    this.saving = true;
    try {
      this.snapshot = await setItemUserCategories(
        this.snapshot.rootPath,
        item.path,
        categoryIds,
      );
      this.selectedItemPath = item.path;
    } catch (error) {
      this.setError(error);
    } finally {
      this.saving = false;
    }
  };

  addUserCategory = async (name: string): Promise<void> => {
    if (!this.snapshot) return;
    try {
      this.snapshot = await createUserCategory(this.snapshot.rootPath, name);
    } catch (error) {
      this.setError(error);
    }
  };

  removeUserCategory = async (id: number): Promise<void> => {
    if (
      !this.snapshot ||
      !window.confirm("이 사용자 카테고리를 삭제하시겠습니까?")
    )
      return;
    try {
      this.snapshot = await deleteUserCategory(this.snapshot.rootPath, id);
      if (this.selectedCategoryId === `${USER_CATEGORY_PREFIX}${id}`) {
        this.selectedCategoryId = null;
      }
    } catch (error) {
      this.setError(error);
    }
  };

  openItem = async (item: ScannedItem): Promise<void> => {
    if (!this.snapshot) return;
    try {
      await launchItem(this.snapshot.rootPath, item.path);
      this.snapshot = await recordItemOpen(this.snapshot.rootPath, item.path);
      this.selectedItemPath = item.path;
    } catch (error) {
      this.setError(error);
    }
  };

  openVideo = async (item: ScannedItem, videoPath: string): Promise<void> => {
    if (!this.snapshot) return;
    try {
      await launchMediaFile(item.path, videoPath);
      this.snapshot = await recordItemOpen(this.snapshot.rootPath, item.path);
      this.selectedItemPath = item.path;
      this.detailPanelOpen = true;
    } catch (error) {
      this.setError(error);
    }
  };

  openItemFolder = async (item: ScannedItem): Promise<void> => {
    try {
      await revealItemInDir(item.path);
    } catch {
      try {
        await openPath(item.path);
      } catch (error) {
        this.setError(error);
      }
    }
  };

  selectCategory(id: string | null): void {
    this.showLibrary();
    this.selectedCategoryId = id;
    if (id === UNCLASSIFIED_CATEGORY_ID) this.selectedRating = null;
    this.selectedItemPath = null;
    this.detailPanelOpen = false;
  }

  selectRating(rating: number | null): void {
    this.showLibrary();
    this.selectedRating = rating;
    this.selectedItemPath = null;
    this.detailPanelOpen = false;
  }

  selectItem(item: ScannedItem): void {
    this.selectedItemPath = item.path;
    this.detailPanelOpen = true;
  }

  selectRandomItem(): void {
    if (!this.visibleItems.length) return;
    this.selectItem(
      this.visibleItems[Math.floor(Math.random() * this.visibleItems.length)],
    );
  }

  moveSelection(direction: number): ScannedItem | null {
    if (!this.visibleItems.length) return null;
    const current = this.visibleItems.findIndex(
      (item) => item.path === this.selectedItemPath,
    );
    const next =
      current < 0
        ? 0
        : (current + direction + this.visibleItems.length) %
          this.visibleItems.length;
    const item = this.visibleItems[next];
    this.selectItem(item);
    return item;
  }

  showLibrary(): void {
    this.activeView = "library";
  }

  showMemo(): void {
    this.activeView = "memo";
    this.detailPanelOpen = false;
    this.statisticsOpen = false;
  }

  setError(error: unknown): void {
    this.errorMessage = error instanceof Error ? error.message : String(error);
  }

  private filterAndSort(items: ScannedItem[]): ScannedItem[] {
    const searched = filterItems(items, this.searchText, this.searchIndex);
    return sortItems(
      filterItemsByRating(searched, this.selectedRating),
      this.sortMode,
    );
  }

  private backgroundReloading = false;
  private backgroundReloadQueued = false;

  private resolveCurrentTitle(): string {
    if (this.selectedCategoryId === null) return "전체 라이브러리";
    if (this.selectedCategoryId === UNCLASSIFIED_CATEGORY_ID)
      return "아직 분류되지 않음";
    if (this.selectedCategoryId === FAVORITES_CATEGORY_ID) return "즐겨찾기";
    if (this.selectedCategoryId === RECENT_CATEGORY_ID) return "최근 실행";
    if (this.selectedCategoryId.startsWith(USER_CATEGORY_PREFIX)) {
      const id = Number(
        this.selectedCategoryId.slice(USER_CATEGORY_PREFIX.length),
      );
      return (
        this.snapshot?.userCategories.find((category) => category.id === id)
          ?.name ?? "사용자 카테고리"
      );
    }
    return this.snapshot
      ? (findCategory(this.snapshot.categories, this.selectedCategoryId)
          ?.name ?? "라이브러리")
      : "라이브러리";
  }
}
