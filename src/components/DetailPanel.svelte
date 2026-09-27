<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { MetadataUpdate, ScannedItem, UserCategory } from "../scripts/types";

  let {
    item,
    userCategories,
    saving,
    onClose,
    onSave,
    onOpen,
    onOpenFolder,
    onSetCategories
  }: {
    item: ScannedItem | null;
    userCategories: UserCategory[];
    saving: boolean;
    onClose: () => void;
    onSave: (update: MetadataUpdate) => void;
    onOpen: (item: ScannedItem) => void;
    onOpenFolder: (item: ScannedItem) => void;
    onSetCategories: (item: ScannedItem, categoryIds: number[]) => void;
  } = $props();

  let favorite = $state(false);
  let rating = $state<number | null>(null);
  let notes = $state("");
  let customThumbnailPath = $state("");
  let tagsText = $state("");
  let selectedCategoryIds = $state<number[]>([]);
  let loadedPath = $state<string | null>(null);

  $effect(() => {
    if (!item || loadedPath === item.path) return;
    loadedPath = item.path;
    favorite = item.favorite;
    rating = item.rating;
    notes = item.notes;
    customThumbnailPath = item.customThumbnailPath ?? "";
    tagsText = item.tags.join(", ");
    selectedCategoryIds = userCategories
      .filter((category) => category.itemPaths.includes(item.path))
      .map((category) => category.id);
  });

  let thumbnail = $derived.by(() => {
    const path = customThumbnailPath.trim() || item?.thumbnailPath;
    return path ? convertFileSrc(path) : null;
  });

  function save(): void {
    if (!item) return;
    onSave({
      path: item.path,
      favorite,
      rating,
      notes,
      customThumbnailPath: customThumbnailPath.trim() || null,
      tags: tagsText.split(",").map((tag) => tag.trim()).filter(Boolean)
    });
    onSetCategories(item, selectedCategoryIds);
  }

  function toggleCategory(id: number): void {
    selectedCategoryIds = selectedCategoryIds.includes(id)
      ? selectedCategoryIds.filter((value) => value !== id)
      : [...selectedCategoryIds, id];
  }
</script>

<aside class="detail">
  {#if item}
    <div class="detail-header"><h2>상세 정보</h2><button onclick={onClose}>×</button></div>

    <div class="cover">
      {#if thumbnail}<img src={thumbnail} alt={item.title} />{:else}<span>{item.itemType}</span>{/if}
    </div>

    <h3>{item.title}</h3>
    {#if item.missing}<p class="warning">현재 폴더에서 찾을 수 없는 작품입니다.</p>{/if}

    <label class="check"><input type="checkbox" bind:checked={favorite} /> 즐겨찾기</label>

    <label>평점
      <select bind:value={rating}>
        <option value={null}>평점 없음</option>
        {#each [0, 1, 2, 3, 4, 5] as score}<option value={score}>{score}점</option>{/each}
      </select>
    </label>

    <label>태그<input bind:value={tagsText} placeholder="태그1, 태그2" /></label>
    <label>사용자 썸네일 경로<input bind:value={customThumbnailPath} placeholder="이미지 파일 경로" /></label>
    <label>메모<textarea bind:value={notes} rows="5"></textarea></label>

    <fieldset>
      <legend>사용자 카테고리</legend>
      {#if userCategories.length === 0}
        <span class="muted">생성된 사용자 카테고리가 없습니다.</span>
      {:else}
        {#each userCategories as category}
          <label class="check">
            <input type="checkbox" checked={selectedCategoryIds.includes(category.id)} onchange={() => toggleCategory(category.id)} />
            {category.name}
          </label>
        {/each}
      {/if}
    </fieldset>

    <dl>
      <dt>포함 파일</dt><dd>{item.fileCount}개</dd>
      <dt>실행 횟수</dt><dd>{item.openCount}회</dd>
      <dt>최근 실행</dt><dd>{item.lastOpenedAt ?? "기록 없음"}</dd>
      <dt>경로</dt><dd>{item.path}</dd>
    </dl>

    <div class="actions">
      <button class="primary" onclick={() => onOpen(item)} disabled={item.missing}>열기</button>
      <button onclick={() => onOpenFolder(item)} disabled={item.missing}>폴더 열기</button>
      <button onclick={save} disabled={saving}>{saving ? "저장 중" : "정보 저장"}</button>
    </div>
  {:else}
    <div class="nothing-selected">작품을 선택하세요.</div>
  {/if}
</aside>

<style>
  .detail { width: 330px; min-width: 330px; height: 100%; padding: 18px; overflow-y: auto; border-left: 1px solid #303036; background: #18181b; }
  .detail-header { display: flex; align-items: center; justify-content: space-between; }
  .detail-header h2 { margin: 0 0 16px; font-size: 16px; }
  .detail-header button { border: 0; color: #a1a1aa; background: transparent; cursor: pointer; font-size: 22px; }
  .cover { width: 100%; aspect-ratio: 5 / 7; display: grid; place-items: center; overflow: hidden; border-radius: 7px; color: #71717a; background: linear-gradient(145deg, #3f3f46, #18181b); }
  .cover img { width: 100%; height: 100%; object-fit: cover; }
  h3 { margin: 16px 0 10px; }
  label { display: flex; flex-direction: column; gap: 6px; margin: 12px 0; color: #a1a1aa; font-size: 12px; }
  label.check { flex-direction: row; align-items: center; color: #d4d4d8; }
  input, select, textarea { width: 100%; border: 1px solid #3f3f46; border-radius: 5px; padding: 8px; color: white; background: #27272a; resize: vertical; }
  .check input { width: auto; }
  fieldset { margin: 16px 0; border: 1px solid #3f3f46; border-radius: 6px; }
  legend { color: #a1a1aa; font-size: 12px; }
  dl { margin: 16px 0; }
  dt { margin-top: 10px; color: #71717a; font-size: 11px; }
  dd { margin: 4px 0 0; overflow-wrap: anywhere; font-size: 13px; }
  .actions { display: grid; gap: 8px; }
  .actions button { min-height: 38px; border: 1px solid #3f3f46; border-radius: 6px; color: white; background: #27272a; cursor: pointer; }
  .actions .primary { border: 0; background: #dc2626; }
  .warning { color: #fca5a5; font-size: 12px; }
  .muted, .nothing-selected { color: #71717a; }
</style>
