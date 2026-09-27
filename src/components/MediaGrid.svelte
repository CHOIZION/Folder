<script lang="ts">
  import MediaCard from "./MediaCard.svelte";

  import type { ScannedItem } from "../scripts/types";

  export interface MediaSection {
    id: string;
    title: string;
    breadcrumb: string;
    isRoot: boolean;
    items: ScannedItem[];
  }

  let {
    title,
    items,
    sections = null,
    selectedItemId,
    loading,
    onSelect,
    onOpen,
    onOpenVideo
  }: {
    title: string;
    items: ScannedItem[];
    sections?: MediaSection[] | null;
    selectedItemId: string | null;
    loading: boolean;
    onSelect: (item: ScannedItem) => void;
    onOpen: (item: ScannedItem) => void;
    onOpenVideo: (item: ScannedItem, videoPath: string) => void;
  } = $props();

  let totalCount = $derived(
    sections
      ? sections.reduce((sum, section) => sum + section.items.length, 0)
      : items.length
  );
</script>

<section class="content" data-heart-scroll-region>
  <div class="content-header">
    <h1>{title}</h1>
    <span>{totalCount}개 항목</span>
  </div>

  {#if loading}
    <div class="empty">폴더를 스캔하고 있습니다.</div>
  {:else if totalCount === 0}
    <div class="empty">표시할 작품이 없습니다.</div>
  {:else if sections}
    <div class="section-list">
      {#each sections as section (section.id)}
        {#if section.items.length > 0}
          <section class:root-section={section.isRoot} class="category-section">
            {#if !section.isRoot}
              <div class="section-header">
                <div class="section-title-row">
                  <span class="section-marker"></span>
                  <h2>{section.breadcrumb || section.title}</h2>
                </div>
                <span>{section.items.length}개</span>
              </div>
            {/if}

            <div class="grid">
              {#each section.items as item (item.id)}
                <MediaCard
                  {item}
                  selected={selectedItemId === item.id}
                  {onSelect}
                  {onOpen}
                  {onOpenVideo}
                />
              {/each}
            </div>
          </section>
        {/if}
      {/each}
    </div>
  {:else}
    <div class="grid">
      {#each items as item (item.id)}
        <MediaCard
          {item}
          selected={selectedItemId === item.id}
          {onSelect}
          {onOpen}
          {onOpenVideo}
        />
      {/each}
    </div>
  {/if}
</section>

<style>
  .content {
    min-width: 0;
    flex: 1;
    padding: 24px;
    overflow-y: auto;
    background: #09090b;
  }

  .content-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 20px;
  }

  h1 {
    margin: 0;
    font-size: 24px;
  }

  .content-header > span,
  .section-header > span {
    color: #a1a1aa;
    font-size: 13px;
  }

  .section-list {
    display: flex;
    flex-direction: column;
    gap: 34px;
  }

  .category-section {
    min-width: 0;
  }

  .root-section + .category-section {
    padding-top: 2px;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin: 0 0 15px;
    padding: 0 2px 10px;
    border-bottom: 1px solid #27272a;
  }

  .section-title-row {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 9px;
  }

  .section-marker {
    width: 4px;
    height: 18px;
    flex: 0 0 auto;
    border-radius: 999px;
    background: #71717a;
  }

  h2 {
    min-width: 0;
    margin: 0;
    overflow: hidden;
    color: #e4e4e7;
    font-size: 17px;
    font-weight: 700;
    line-height: 1.3;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 18px;
  }

  .empty {
    height: 300px;
    display: grid;
    place-items: center;
    color: #71717a;
  }
</style>
