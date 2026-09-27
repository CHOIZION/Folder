<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { ScannedItem } from "../scripts/types";
  import {
    getTagKind,
    getTagLabel,
    type TagKind
  } from "../scripts/tags";

  let {
    item,
    selected,
    onSelect,
    onOpen,
    onOpenVideo
  }: {
    item: ScannedItem;
    selected: boolean;
    onSelect: (item: ScannedItem) => void;
    onOpen: (item: ScannedItem) => void;
    onOpenVideo: (item: ScannedItem, videoPath: string) => void;
  } = $props();

  let thumbnail = $derived.by(() => {
    const path = item.customThumbnailPath ?? item.thumbnailPath;
    return path ? convertFileSrc(path) : null;
  });

  let videos = $derived(item.videoFiles ?? []);
  let hasEpisodePicker = $derived(item.itemType === "video" && videos.length > 1);

  let groupedTags = $derived.by(() => ({
    genre: getTags("genre"),
    topic: getTags("topic"),
    etc: getTags("etc")
  }));

  function getTypeText(type: ScannedItem["itemType"]): string {
    return {
      game: "게임",
      video: "영상",
      image: "이미지",
      document: "문서",
      archive: "압축",
      folder: "폴더"
    }[type];
  }

  function getTags(kind: TagKind): string[] {
    return item.tags
      .filter((tag) => getTagKind(tag) === kind)
      .map(getTagLabel)
      .filter(Boolean);
  }

  function videoName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

  function openEpisode(event: MouseEvent, videoPath: string): void {
    event.preventDefault();
    event.stopPropagation();
    onSelect(item);
    onOpenVideo(item, videoPath);
  }
</script>

<div
  class="card"
  role="button"
  tabindex="0"
  class:selected
  class:missing={item.missing}
  onclick={() => onSelect(item)}
  ondblclick={(event) => { event.preventDefault(); onOpen(item); }}
  onkeydown={(event) => {
    if (event.key === " " || event.key === "Spacebar") {
      event.preventDefault();
      onSelect(item);
    }
  }}
>
  <div class="thumbnail">
    {#if thumbnail}
      <img src={thumbnail} alt={item.title} />
    {:else}
      <span>{getTypeText(item.itemType)}</span>
    {/if}

    {#if item.favorite}<span class="favorite">★</span>{/if}

    {#if hasEpisodePicker}
      <div class="episode-strip" aria-label={`${videos.length}개 영상`}>
        {#each videos as video, index}
          <button
            type="button"
            class="episode-button"
            title={`${index + 1}. ${videoName(video)}`}
            aria-label={`${index + 1}번째 영상 재생: ${videoName(video)}`}
            onclick={(event) => openEpisode(event, video)}
            ondblclick={(event) => { event.preventDefault(); event.stopPropagation(); }}
          >
            {index + 1}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <div class="card-content">
    <strong title={item.title}>{item.title}</strong>

    <div class="tag-groups" aria-label="태그">
      {#each [
        { kind: "genre" as const, symbol: "▣", tags: groupedTags.genre },
        { kind: "topic" as const, symbol: "◎", tags: groupedTags.topic },
        { kind: "etc" as const, symbol: "◇", tags: groupedTags.etc }
      ] as group}
        <div class="tag-row {group.kind}" title={group.tags.join(", ")}>
          <span class="tag-symbol">{group.symbol}</span>
          {#if group.tags.length > 0}
            {#each group.tags.slice(0, 2) as tag}
              <span class="tag-chip">{tag}</span>
            {/each}
            {#if group.tags.length > 2}
              <span class="tag-more">+{group.tags.length - 2}</span>
            {/if}
          {:else}
            <span class="tag-empty">—</span>
          {/if}
        </div>
      {/each}
    </div>

    <div class="rating-stars" aria-label={`별점 ${item.rating ?? 0}점`}>
      {#each [1, 2, 3, 4, 5] as star}
        <span class:filled={(item.rating ?? 0) >= star}>★</span>
      {/each}
    </div>

    <div class="metadata">
      <span>{item.fileCount}개 파일</span>
      {#if hasEpisodePicker}<span>{videos.length}편</span>{/if}
    </div>
  </div>
</div>

<style>
  .card { width: 100%; padding: 0; overflow: hidden; border: 1px solid transparent; border-radius: 8px; text-align: left; color: #f4f4f5; background: #27272a; cursor: pointer; transition: transform 120ms ease, border-color 120ms ease; }
  .card:hover { transform: translateY(-3px); border-color: #52525b; }
  .card.selected { border-color: #ef4444; box-shadow: 0 0 0 1px #ef4444; }
  .card.missing { opacity: .55; }
  .thumbnail { position: relative; aspect-ratio: 5 / 7; display: grid; place-items: center; overflow: hidden; background: linear-gradient(145deg, #3f3f46, #18181b); color: #71717a; font-weight: 700; }
  img { width: 100%; height: 100%; object-fit: cover; }
  .favorite { position: absolute; top: 8px; left: 8px; padding: 3px 7px; border-radius: 4px; color: #facc15; background: rgb(0 0 0 / 65%); }
  .episode-strip { position: absolute; right: 7px; bottom: 7px; left: 7px; display: flex; gap: 5px; padding: 6px; overflow-x: auto; border: 1px solid rgb(255 255 255 / 12%); border-radius: 10px; background: rgb(9 9 11 / 82%); backdrop-filter: blur(5px); scrollbar-width: thin; }
  .episode-button { min-width: 27px; height: 27px; flex: 0 0 27px; padding: 0; border: 1px solid #71717a; border-radius: 999px; color: #fafafa; background: #27272a; cursor: pointer; font-size: 11px; font-weight: 800; transition: border-color 120ms ease, background 120ms ease, transform 120ms ease; }
  .episode-button:hover { border-color: #f87171; background: #dc2626; transform: translateY(-1px); }
  .card-content { height: 181px; padding: 12px; display: flex; flex-direction: column; }
  strong { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  strong { min-height: 19px; }
  .tag-groups { height: 72px; margin-top: 10px; display: grid; grid-template-rows: repeat(3, 22px); gap: 3px; overflow: hidden; }
  .tag-row { min-width: 0; display: flex; align-items: center; gap: 4px; overflow: hidden; white-space: nowrap; }
  .tag-symbol { width: 15px; flex: 0 0 15px; text-align: center; font-size: 12px; font-weight: 800; }
  .tag-chip { min-width: 0; max-width: 40%; padding: 2px 6px; overflow: hidden; border: 1px solid currentColor; border-radius: 999px; text-overflow: ellipsis; white-space: nowrap; font-size: 10px; line-height: 1.25; background: rgb(255 255 255 / 5%); }
  .tag-more { flex: 0 0 auto; font-size: 10px; font-weight: 700; }
  .tag-empty { color: #52525b; font-size: 11px; }
  .tag-row.genre { color: #93c5fd; }
  .tag-row.topic { color: #f9a8d4; }
  .tag-row.etc { color: #a1a1aa; }
  .rating-stars { margin-top: 7px; display: flex; gap: 1px; color: #52525b; font-size: 14px; line-height: 1; }
  .rating-stars span.filled { color: #facc15; }
  .metadata { margin-top: auto; display: flex; justify-content: space-between; color: #71717a; font-size: 12px; }
</style>
