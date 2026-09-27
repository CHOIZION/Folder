<script lang="ts">
  import type { ScannedItem } from "../scripts/types";
  import {
    getTagKind,
    getTagLabel,
    type TagKind
  } from "../scripts/tags";

  let {
    items,
    onClose
  }: {
    items: ScannedItem[];
    onClose: () => void;
  } = $props();

  type TagCount = { name: string; count: number };

  let favoriteCount = $derived(items.filter((item) => item.favorite).length);
  let ratedItems = $derived(items.filter((item) => item.rating !== null));
  let averageRating = $derived(
    ratedItems.length === 0
      ? 0
      : ratedItems.reduce((sum, item) => sum + (item.rating ?? 0), 0) / ratedItems.length
  );
  let playedCount = $derived(items.filter((item) => item.openCount > 0).length);
  let ratingCounts = $derived([1, 2, 3, 4, 5].map((rating) => ({
    rating,
    count: items.filter((item) => item.rating === rating).length
  })));
  let genreTags = $derived(countTags("genre"));
  let topicTags = $derived(countTags("topic"));
  let etcTags = $derived(countTags("etc"));

  function countTags(kind: TagKind): TagCount[] {
    const counts = new Map<string, number>();
    for (const item of items) {
      for (const tag of item.tags) {
        if (getTagKind(tag) !== kind) continue;
        const name = getTagLabel(tag);
        if (!name) continue;
        counts.set(name, (counts.get(name) ?? 0) + 1);
      }
    }
    return [...counts.entries()]
      .map(([name, count]) => ({ name, count }))
      .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name, "ko-KR"))
      .slice(0, 10);
  }

  function maxCount(values: TagCount[]): number {
    return Math.max(1, ...values.map((value) => value.count));
  }
</script>

<div class="overlay" role="presentation" onclick={(event) => event.currentTarget === event.target && onClose()}>
  <dialog class="panel" open aria-label="라이브러리 통계">
    <header>
      <div>
        <h2>라이브러리 통계</h2>
        <p>DB에 저장된 현재 라이브러리 메타데이터 기준입니다.</p>
      </div>
      <button class="close" onclick={onClose} aria-label="통계 닫기">×</button>
    </header>

    <div class="summary">
      <article><span>전체 작품</span><strong>{items.length}</strong></article>
      <article><span>즐겨찾기</span><strong>{favoriteCount}</strong></article>
      <article><span>평균 별점</span><strong>{averageRating.toFixed(1)}</strong></article>
      <article><span>실행한 작품</span><strong>{playedCount}</strong></article>
    </div>

    <div class="rating-card">
      <h3>별점 분포</h3>
      <div class="rating-bars">
        {#each ratingCounts as entry}
          <div class="rating-entry">
            <span>{entry.rating}점</span>
            <div class="bar-track"><div class="bar rating" style={`width: ${items.length ? entry.count / items.length * 100 : 0}%`}></div></div>
            <strong>{entry.count}</strong>
          </div>
        {/each}
      </div>
    </div>

    <div class="tag-sections">
      {#each [
        { title: "장르 태그 TOP 10", kind: "genre", values: genreTags },
        { title: "주제 태그 TOP 10", kind: "topic", values: topicTags },
        { title: "기타 태그 TOP 10", kind: "etc", values: etcTags }
      ] as group}
        <article class="tag-card {group.kind}">
          <h3>{group.title}</h3>
          {#if group.values.length === 0}
            <p class="empty">저장된 태그가 없습니다.</p>
          {:else}
            <div class="bars">
              {#each group.values as entry, index}
                <div class="bar-entry">
                  <span class="rank">{index + 1}</span>
                  <span class="name" title={entry.name}>{entry.name}</span>
                  <div class="bar-track"><div class="bar" style={`width: ${entry.count / maxCount(group.values) * 100}%`}></div></div>
                  <strong>{entry.count}</strong>
                </div>
              {/each}
            </div>
          {/if}
        </article>
      {/each}
    </div>
  </dialog>
</div>

<style>
  .overlay { position: fixed; z-index: 100; inset: 0; display: grid; place-items: center; padding: 28px; background: rgb(0 0 0 / 72%); backdrop-filter: blur(4px); }
  .panel { width: min(1180px, 96vw); max-height: 92vh; overflow-y: auto; padding: 24px; border: 1px solid #3f3f46; border-radius: 12px; background: #18181b; box-shadow: 0 24px 80px rgb(0 0 0 / 65%); }
  header { display: flex; align-items: flex-start; justify-content: space-between; gap: 18px; }
  h2, h3, p { margin: 0; }
  h2 { font-size: 23px; }
  header p { margin-top: 6px; color: #71717a; font-size: 12px; }
  .close { width: 36px; height: 36px; border: 0; color: #a1a1aa; background: transparent; cursor: pointer; font-size: 27px; }
  .summary { margin-top: 20px; display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; }
  .summary article, .rating-card, .tag-card { border: 1px solid #303036; border-radius: 9px; background: #202023; }
  .summary article { padding: 16px; }
  .summary span { display: block; color: #a1a1aa; font-size: 12px; }
  .summary strong { display: block; margin-top: 7px; font-size: 24px; }
  .rating-card { margin-top: 12px; padding: 17px; }
  .rating-card h3, .tag-card h3 { margin-bottom: 15px; font-size: 14px; }
  .rating-entry, .bar-entry { min-width: 0; display: grid; align-items: center; gap: 9px; margin: 9px 0; }
  .rating-entry { grid-template-columns: 34px 1fr 34px; }
  .bar-entry { grid-template-columns: 20px minmax(70px, 110px) 1fr 28px; }
  .rank { color: #71717a; font-size: 11px; }
  .name { overflow: hidden; color: #d4d4d8; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; }
  .bar-track { height: 9px; overflow: hidden; border-radius: 999px; background: #3f3f46; }
  .bar { height: 100%; min-width: 2px; border-radius: inherit; background: currentColor; }
  .bar.rating { color: #facc15; }
  .rating-entry > span, .rating-entry strong, .bar-entry strong { font-size: 11px; }
  .tag-sections { margin-top: 12px; display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }
  .tag-card { min-width: 0; padding: 17px; }
  .tag-card.genre { color: #60a5fa; }
  .tag-card.topic { color: #f472b6; }
  .tag-card.etc { color: #a1a1aa; }
  .tag-card h3 { color: #f4f4f5; }
  .empty { color: #71717a; font-size: 12px; }
  @media (max-width: 900px) {
    .summary { grid-template-columns: repeat(2, 1fr); }
    .tag-sections { grid-template-columns: 1fr; }
  }
</style>
