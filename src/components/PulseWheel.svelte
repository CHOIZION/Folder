<script lang="ts">
  import { onMount, tick } from "svelte";

  let {
    x,
    y,
    totalCount,
    favoriteCount,
    recentCount,
    oldestTitle,
    lastTitle,
    onSearch,
    onOldest,
    onRecent,
    onRandom,
    onAll,
    onFavorites,
    onLast,
    onClose
  }: {
    x: number;
    y: number;
    totalCount: number;
    favoriteCount: number;
    recentCount: number;
    oldestTitle: string | null;
    lastTitle: string | null;
    onSearch: (query: string) => void;
    onOldest: () => void;
    onRecent: () => void;
    onRandom: () => void;
    onAll: () => void;
    onFavorites: () => void;
    onLast: () => void;
    onClose: () => void;
  } = $props();

  let query = $state("");
  let searchInput: HTMLInputElement | null = null;

  onMount(() => {
    void tick().then(() => searchInput?.focus());
  });

  function submitSearch(): void {
    const value = query.trim();
    if (!value) return;
    onSearch(value);
  }

  function stop(event: MouseEvent): void {
    event.stopPropagation();
  }
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    }
  }}
/>

<div
  class="pulse-backdrop"
  role="presentation"
  onmousedown={(event) => event.currentTarget === event.target && onClose()}
>
  <div
    class="pulse-wheel"
    style={`left: ${x}px; top: ${y}px`}
    role="dialog"
    aria-modal="true"
    aria-label="HEART PULSE 빠른 탐색"
    tabindex="-1"
    onmousedown={stop}
  >
    <div class="orbit" aria-hidden="true"></div>
    <div class="pulse-mark" aria-hidden="true">PULSE</div>

    <button
      class="action oldest"
      type="button"
      title={oldestTitle ? `가장 오래전에 실행: ${oldestTitle}` : "실행 기록이 없습니다"}
      disabled={!oldestTitle}
      onclick={onOldest}
    >
      <span class="icon">↺</span>
      <strong>오랜만</strong>
      <small>{oldestTitle ?? "기록 없음"}</small>
    </button>

    <button
      class="action recent"
      type="button"
      disabled={recentCount === 0}
      onclick={onRecent}
    >
      <span class="icon">◷</span>
      <strong>최근 실행</strong>
      <small>{recentCount}개</small>
    </button>

    <button
      class="action random"
      type="button"
      disabled={totalCount === 0}
      onclick={onRandom}
    >
      <span class="icon">⤨</span>
      <strong>랜덤</strong>
      <small>전체 {totalCount}개</small>
    </button>

    <button
      class="action all"
      type="button"
      disabled={totalCount === 0}
      onclick={onAll}
    >
      <span class="icon">▦</span>
      <strong>전체 보기</strong>
      <small>{totalCount}개</small>
    </button>

    <button
      class="action favorites"
      type="button"
      disabled={favoriteCount === 0}
      onclick={onFavorites}
    >
      <span class="icon favorite-icon">★</span>
      <strong>즐겨찾기</strong>
      <small>{favoriteCount}개</small>
    </button>

    <button
      class="action last"
      type="button"
      title={lastTitle ? `마지막으로 실행: ${lastTitle}` : "실행 기록이 없습니다"}
      disabled={!lastTitle}
      onclick={onLast}
    >
      <span class="icon">▶</span>
      <strong>마지막 작품</strong>
      <small>{lastTitle ?? "기록 없음"}</small>
    </button>

    <form
      class="search-core"
      onsubmit={(event) => {
        event.preventDefault();
        submitSearch();
      }}
    >
      <span class="heart">♥</span>
      <label for="pulse-search">바로 검색</label>
      <input
        id="pulse-search"
        bind:this={searchInput}
        bind:value={query}
        type="search"
        autocomplete="off"
        placeholder="제목 · 태그 · 메모"
      />
      <small>입력 후 Enter</small>
    </form>

    <div class="hint">휠 클릭으로 열기 · Esc로 닫기</div>
  </div>
</div>

<style>
  .pulse-backdrop {
    position: fixed;
    z-index: 240;
    inset: 0;
    overflow: hidden;
    background:
      radial-gradient(circle at center, rgb(127 29 29 / 16%), transparent 42%),
      rgb(0 0 0 / 64%);
    backdrop-filter: blur(5px);
    animation: backdrop-in 120ms ease-out;
  }

  .pulse-wheel {
    position: absolute;
    width: 382px;
    height: 382px;
    transform: translate(-50%, -50%) scale(1);
    animation: wheel-in 150ms cubic-bezier(.2, .9, .25, 1.1);
  }

  .orbit {
    position: absolute;
    inset: 34px;
    border: 1px solid rgb(248 113 113 / 27%);
    border-radius: 50%;
    background:
      radial-gradient(circle, rgb(24 24 27 / 92%) 0 31%, transparent 32%),
      conic-gradient(from 30deg, rgb(127 29 29 / 12%), transparent 13%, rgb(127 29 29 / 12%) 17%, transparent 30%);
    box-shadow: 0 0 70px rgb(220 38 38 / 13%), inset 0 0 50px rgb(0 0 0 / 42%);
  }

  .orbit::before,
  .orbit::after {
    content: "";
    position: absolute;
    inset: 36px;
    border: 1px dashed rgb(161 161 170 / 16%);
    border-radius: 50%;
  }

  .orbit::after {
    inset: 92px;
    border-style: solid;
    border-color: rgb(248 113 113 / 20%);
  }

  .pulse-mark {
    position: absolute;
    top: 17px;
    left: 50%;
    z-index: 2;
    transform: translateX(-50%);
    color: #ef4444;
    font-size: 9px;
    font-weight: 800;
    letter-spacing: 3px;
  }

  .action {
    position: absolute;
    z-index: 3;
    width: 112px;
    height: 70px;
    display: grid;
    grid-template-columns: 26px minmax(0, 1fr);
    grid-template-rows: 22px 18px;
    align-content: center;
    align-items: center;
    gap: 0 7px;
    padding: 9px 10px;
    border: 1px solid #3f3f46;
    border-radius: 12px;
    color: #e4e4e7;
    background: linear-gradient(145deg, rgb(39 39 42 / 98%), rgb(24 24 27 / 98%));
    box-shadow: 0 10px 24px rgb(0 0 0 / 35%);
    cursor: pointer;
    text-align: left;
    transition: border-color 120ms ease, background 120ms ease, transform 120ms ease, box-shadow 120ms ease;
  }

  .action:hover:not(:disabled),
  .action:focus-visible {
    z-index: 5;
    border-color: #ef4444;
    outline: none;
    color: white;
    background: linear-gradient(145deg, #4c1d22, #27272a);
    box-shadow: 0 10px 28px rgb(220 38 38 / 24%);
  }

  .action:disabled {
    cursor: default;
    opacity: .38;
  }

  .action .icon {
    grid-row: 1 / 3;
    width: 25px;
    height: 25px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: #fca5a5;
    background: rgb(127 29 29 / 42%);
    font-size: 13px;
    font-weight: 800;
  }

  .action .favorite-icon { color: #fde047; }
  .action strong,
  .action small { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .action strong { font-size: 12px; }
  .action small { color: #71717a; font-size: 9px; }

  .oldest { top: 0; left: 50%; transform: translateX(-50%); }
  .oldest:hover:not(:disabled) { transform: translateX(-50%) translateY(-3px); }
  .recent { top: 74px; right: 0; }
  .recent:hover:not(:disabled) { transform: translate(3px, -2px); }
  .random { right: 0; bottom: 74px; }
  .random:hover:not(:disabled) { transform: translate(3px, 2px); }
  .all { bottom: 0; left: 50%; transform: translateX(-50%); }
  .all:hover:not(:disabled) { transform: translateX(-50%) translateY(3px); }
  .favorites { bottom: 74px; left: 0; }
  .favorites:hover:not(:disabled) { transform: translate(-3px, 2px); }
  .last { top: 74px; left: 0; }
  .last:hover:not(:disabled) { transform: translate(-3px, -2px); }

  .search-core {
    position: absolute;
    z-index: 4;
    top: 50%;
    left: 50%;
    width: 160px;
    height: 160px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 5px;
    transform: translate(-50%, -50%);
    border: 1px solid #52525b;
    border-radius: 50%;
    background: radial-gradient(circle at 50% 35%, #323238, #18181b 68%);
    box-shadow: 0 16px 45px rgb(0 0 0 / 52%), 0 0 0 7px rgb(220 38 38 / 6%);
  }

  .heart {
    height: 20px;
    color: #ef4444;
    font-size: 19px;
    line-height: 1;
    filter: drop-shadow(0 0 8px rgb(239 68 68 / 40%));
  }

  .search-core label { color: #f4f4f5; font-size: 11px; font-weight: 800; }

  .search-core input {
    width: 124px;
    height: 29px;
    padding: 0 9px;
    border: 1px solid #3f3f46;
    border-radius: 999px;
    outline: none;
    color: white;
    background: #09090b;
    font-size: 10px;
    text-align: center;
  }

  .search-core input:focus { border-color: #ef4444; box-shadow: 0 0 0 3px rgb(239 68 68 / 10%); }
  .search-core input::placeholder { color: #52525b; }
  .search-core small { color: #52525b; font-size: 8px; }

  .hint {
    position: absolute;
    top: calc(100% + 15px);
    left: 50%;
    transform: translateX(-50%);
    color: #71717a;
    font-size: 10px;
    white-space: nowrap;
  }

  @keyframes backdrop-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes wheel-in {
    from { opacity: 0; transform: translate(-50%, -50%) scale(.88); }
    to { opacity: 1; transform: translate(-50%, -50%) scale(1); }
  }

  @media (max-width: 760px), (max-height: 620px) {
    .pulse-wheel { transform: translate(-50%, -50%) scale(.86); }
    @keyframes wheel-in {
      from { opacity: 0; transform: translate(-50%, -50%) scale(.76); }
      to { opacity: 1; transform: translate(-50%, -50%) scale(.86); }
    }
  }
</style>
