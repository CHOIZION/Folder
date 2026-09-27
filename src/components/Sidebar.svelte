<script lang="ts">
  import type { Category, UserCategory } from "../scripts/types";
  import {
    FAVORITES_CATEGORY_ID,
    RECENT_CATEGORY_ID,
    UNCLASSIFIED_CATEGORY_ID,
    USER_CATEGORY_PREFIX
  } from "../scripts/library";

  let {
    categories,
    userCategories,
    selectedCategoryId,
    selectedRating,
    unclassifiedCount,
    favoriteCount,
    recentCount,
    ratingCounts,
    totalCount,
    onSelect,
    onSelectRating,
    onCreateUserCategory,
    onDeleteUserCategory,
    memoActive,
    onOpenMemo
  }: {
    categories: Category[];
    userCategories: UserCategory[];
    selectedCategoryId: string | null;
    selectedRating: number | null;
    unclassifiedCount: number;
    favoriteCount: number;
    recentCount: number;
    ratingCounts: number[];
    totalCount: number;
    onSelect: (categoryId: string | null) => void;
    onSelectRating: (rating: number | null) => void;
    onCreateUserCategory: (name: string) => void;
    onDeleteUserCategory: (id: number) => void;
    memoActive: boolean;
    onOpenMemo: () => void;
  } = $props();

  let expandedIds = $state<string[]>([]);
  let newCategoryName = $state("");

  type VisibleCategory = { category: Category; depth: number };

  let visibleCategories = $derived.by(() => {
    const result: VisibleCategory[] = [];
    function add(category: Category, depth: number): void {
      result.push({ category, depth });
      if (expandedIds.includes(category.id)) {
        category.children.forEach((child) => add(child, depth + 1));
      }
    }
    categories.forEach((category) => add(category, 0));
    return result;
  });

  function toggleCategory(id: string): void {
    expandedIds = expandedIds.includes(id)
      ? expandedIds.filter((value) => value !== id)
      : [...expandedIds, id];
  }

  function countItems(category: Category): number {
    return category.items.length + category.children.reduce(
      (total, child) => total + countItems(child),
      0
    );
  }

  function submitCategory(): void {
    const name = newCategoryName.trim();
    if (!name) return;
    onCreateUserCategory(name);
    newCategoryName = "";
  }
</script>

<aside class="sidebar">
  <div class="system-list">
    <button class:active={selectedCategoryId === null} onclick={() => onSelect(null)}>
      <span>전체 라이브러리</span><small>{totalCount}</small>
    </button>
    <button class:active={selectedCategoryId === FAVORITES_CATEGORY_ID} onclick={() => onSelect(FAVORITES_CATEGORY_ID)}>
      <span>즐겨찾기</span><small>{favoriteCount}</small>
    </button>
    <button class:active={selectedCategoryId === RECENT_CATEGORY_ID} onclick={() => onSelect(RECENT_CATEGORY_ID)}>
      <span>최근 실행</span><small>{recentCount}</small>
    </button>
    <button class:active={selectedCategoryId === UNCLASSIFIED_CATEGORY_ID} onclick={() => onSelect(UNCLASSIFIED_CATEGORY_ID)}>
      <span>아직 분류되지 않음</span><small>{unclassifiedCount}</small>
    </button>
  </div>

  <div class="divider"></div>
  <div class="section-title">별점</div>
  <div class="rating-list">
    <button class:active={selectedRating === null} onclick={() => onSelectRating(null)}>
      <span>전체 별점</span><small>{totalCount}</small>
    </button>
    {#each [5, 4, 3, 2, 1] as rating}
      <button class:active={selectedRating === rating} onclick={() => onSelectRating(rating)} aria-label={`${rating}점만 보기`}>
        <span class="stars">
          {#each [1, 2, 3, 4, 5] as star}
            <span class:filled={star <= rating}>★</span>
          {/each}
        </span>
        <small>{ratingCounts[rating] ?? 0}</small>
      </button>
    {/each}
  </div>

  <div class="divider"></div>
  <div class="section-title">폴더 카테고리</div>

  <div class="category-list">
    {#each visibleCategories as row (row.category.id)}
      <div class="category-row" class:selected={selectedCategoryId === row.category.id} style={`padding-left:${10 + row.depth * 18}px`}>
        {#if row.category.children.length > 0}
          <button class="arrow" aria-label="펼치기" onclick={() => toggleCategory(row.category.id)}>
            {expandedIds.includes(row.category.id) ? "▼" : "▶"}
          </button>
        {:else}
          <span class="arrow-space"></span>
        {/if}
        <button class="category-button" title={row.category.path} onclick={() => onSelect(row.category.id)}>
          <span>{row.category.name}</span><small>{countItems(row.category)}</small>
        </button>
      </div>
    {/each}
  </div>

  <div class="divider"></div>
  <div class="section-title">사용자 카테고리</div>

  <form onsubmit={(event) => { event.preventDefault(); submitCategory(); }}>
    <input bind:value={newCategoryName} placeholder="새 카테고리" />
    <button type="submit">추가</button>
  </form>

  <div class="user-list">
    {#each userCategories as category (category.id)}
      <div class="user-row" class:selected={selectedCategoryId === `${USER_CATEGORY_PREFIX}${category.id}`}>
        <button class="user-select" onclick={() => onSelect(`${USER_CATEGORY_PREFIX}${category.id}`)}>
          <span>{category.name}</span><small>{category.itemPaths.length}</small>
        </button>
        <button class="delete" aria-label="삭제" onclick={() => onDeleteUserCategory(category.id)}>×</button>
      </div>
    {/each}
  </div>

  <div class="sidebar-footer">
    <div class="footer-divider"></div>
    <button class="footer-button" class:active={memoActive} onclick={onOpenMemo}>
      <span>메모장</span><span class="memo-mark">✎</span>
    </button>
  </div>
</aside>

<style>
  .sidebar { position: relative; width: 280px; min-width: 280px; height: 100%; overflow-y: auto; border-right: 1px solid #303036; background: #18181b; padding-bottom: 64px; }
  .system-list, .rating-list { padding: 10px 8px; }
  .rating-list { padding-top: 0; }
  .system-list button, .rating-list button, .category-button, .user-select { width: 100%; min-height: 38px; display: flex; align-items: center; justify-content: space-between; border: 0; border-radius: 5px; padding: 0 12px; color: #d4d4d8; background: transparent; cursor: pointer; text-align: left; }
  .system-list button:hover, .system-list button.active, .rating-list button:hover, .rating-list button.active, .category-row:hover, .category-row.selected, .user-row:hover, .user-row.selected { color: white; background: #27272a; }
  small { color: #71717a; }
  .stars { display: flex; gap: 1px; color: #52525b; font-size: 14px; }
  .stars span.filled { color: #facc15; }
  .divider { height: 1px; margin: 8px 12px; background: #303036; }
  .section-title { padding: 5px 14px 8px; color: #71717a; font-size: 12px; font-weight: 700; }
  .category-row, .user-row { min-height: 36px; display: flex; align-items: center; }
  .arrow { width: 22px; min-width: 22px; height: 36px; border: 0; color: #a1a1aa; background: transparent; cursor: pointer; font-size: 10px; }
  .arrow-space { width: 22px; min-width: 22px; }
  .category-button { min-width: 0; flex: 1; padding-left: 4px; }
  .category-button span, .user-select span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  form { display: flex; gap: 5px; padding: 0 8px 8px; }
  form input { min-width: 0; flex: 1; height: 34px; border: 1px solid #3f3f46; border-radius: 5px; padding: 0 8px; color: white; background: #27272a; }
  form button { width: 48px; border: 0; border-radius: 5px; color: white; background: #dc2626; cursor: pointer; }
  .user-select { min-width: 0; flex: 1; }
  .delete { width: 32px; height: 32px; border: 0; color: #a1a1aa; background: transparent; cursor: pointer; font-size: 18px; }
  .sidebar-footer { position: fixed; z-index: 20; left: 0; bottom: 0; width: 280px; padding: 5px 8px 9px; border-right: 1px solid #303036; background: #18181b; box-shadow: 0 -12px 20px rgba(24,24,27,.95); }
  .footer-divider { height: 1px; margin: 4px 4px; background: #303036; }
  .footer-button { width: 100%; height: 40px; display: flex; align-items: center; justify-content: space-between; border: 0; border-radius: 6px; padding: 0 12px; color: #d4d4d8; background: transparent; cursor: pointer; text-align: left; }
  .footer-button:hover, .footer-button.active { color: white; background: #29292e; }
  .memo-mark { color: #a1a1aa; font-size: 16px; }
</style>
