<script lang="ts">
  import TagSearch from "./TagSearch.svelte";
  import type { SortMode } from "../scripts/library";
  import type { TagCatalogEntry } from "../scripts/tags";

  let {
    searchText,
    tagCatalog,
    rootPath,
    databasePath,
    scanning,
    sortMode,
    onSearch,
    onRescan,
    onChooseRoot,
    onSort,
    onRandom,
    onStatistics,
    remoteConnected,
    remotePaired,
    onRemote,
    assistantActive,
    onAssistant,
    minimizeToTray,
    savingPreferences,
    onMinimizeToTray
  }: {
    searchText: string;
    tagCatalog: TagCatalogEntry[];
    rootPath: string;
    databasePath: string;
    scanning: boolean;
    sortMode: SortMode;
    onSearch: (value: string) => void;
    onRescan: () => void;
    onChooseRoot: (path: string) => void;
    onSort: (mode: SortMode) => void;
    onRandom: () => void;
    onStatistics: () => void;
    remoteConnected: boolean;
    remotePaired: boolean;
    onRemote: () => void;
    assistantActive: boolean;
    onAssistant: () => void;
    minimizeToTray: boolean;
    savingPreferences: boolean;
    onMinimizeToTray: (enabled: boolean) => void;
  } = $props();

  let menuOpen = $state(false);
  let rootDraft = $state("");
  $effect(() => { rootDraft = rootPath; });

  const sortLabels: Record<SortMode, string> = {
    "abc-asc": "ABC 정렬",
    "abc-desc": "ABC 정렬 역순",
    "ko-asc": "가나다 정렬",
    "ko-desc": "가나다 정렬 역순"
  };

  function chooseSort(mode: SortMode): void {
    onSort(mode);
    menuOpen = false;
  }
</script>

<header class="topbar">
  <div class="brand-area">
    <div class="settings-wrap">
      <button class="settings-button" class:active={menuOpen} aria-label="설정 메뉴" title="설정" onclick={() => menuOpen = !menuOpen}>
        <img src="/heart.png" alt="" />
      </button>

      {#if menuOpen}
        <div class="settings-menu">
          <div class="submenu-row">
            <button class="menu-entry">
              <span>정렬</span><span>›</span>
            </button>
            <div class="sort-submenu">
              {#each Object.entries(sortLabels) as [mode, label]}
                <button class:selected={sortMode === mode} onclick={() => chooseSort(mode as SortMode)}>{label}</button>
              {/each}
            </div>
          </div>
          <button class="menu-entry" onclick={() => { onRandom(); menuOpen = false; }}>
            <span>랜덤 선택</span><kbd>R</kbd>
          </button>
          <button class="menu-entry" onclick={() => { onStatistics(); menuOpen = false; }}>
            <span>통계</span><span>▥</span>
          </button>
          <div class="menu-divider"></div>
          <button
            class="menu-entry"
            role="switch"
            aria-checked={minimizeToTray}
            disabled={savingPreferences}
            onclick={() => onMinimizeToTray(!minimizeToTray)}
          >
            <span>닫을 때 트레이로 이동</span>
            <span class:enabled={minimizeToTray} class="switch"><i></i></span>
          </button>
        </div>
      {/if}
    </div>
    <div class="brand">HEART</div>
  </div>

  <div class="paths">
    <form onsubmit={(event) => { event.preventDefault(); onChooseRoot(rootDraft); }}>
      <input aria-label="라이브러리 폴더 경로" placeholder="라이브러리 폴더 경로 입력" bind:value={rootDraft} disabled={scanning} />
      <button type="submit" disabled={scanning || !rootDraft.trim()}>열기</button>
    </form>
    <small title={databasePath}>{databasePath || "DB 준비 중"}</small>
  </div>
  <TagSearch value={searchText} catalog={tagCatalog} onInput={onSearch} />
  <button class:active={assistantActive} class="assistant-button" onclick={onAssistant}>
    <span>◉</span> HEART Assistant
  </button>
  <button class:connected={remoteConnected} class:paired={remotePaired} class="remote-button" onclick={onRemote} title="Android HEART Remote 연결">
    <span class="remote-dot"></span> REMOTE
  </button>
  <button class="rescan" onclick={onRescan} disabled={scanning || !rootPath}>{scanning ? "스캔 중..." : "다시 스캔"}</button>
</header>

<style>
  .topbar { position: relative; z-index: 20; height: 64px; display: flex; align-items: center; gap: 16px; padding: 0 20px; border-bottom: 1px solid #303036; background: #18181b; }
  .brand-area { width: 148px; flex-shrink: 0; display: flex; align-items: center; gap: 11px; }
  .brand { font-size: 22px; font-weight: 800; letter-spacing: 2px; }
  .settings-wrap { position: relative; }
  .settings-button { width: 34px; height: 34px; display: grid; place-items: center; padding: 0; border: 1px solid transparent; border-radius: 7px; background: transparent; cursor: pointer; }
  .settings-button:hover, .settings-button.active { border-color: #52525b; background: #27272a; }
  .settings-button img { width: 25px; height: 25px; object-fit: contain; }
  .settings-menu { position: absolute; top: 42px; left: 0; width: 228px; padding: 6px; border: 1px solid #3f3f46; border-radius: 8px; background: #202023; box-shadow: 0 14px 35px rgb(0 0 0 / 45%); }
  .menu-entry, .sort-submenu button { width: 100%; min-height: 36px; display: flex; align-items: center; justify-content: space-between; padding: 0 10px; border: 0; border-radius: 5px; color: #e4e4e7; background: transparent; cursor: pointer; text-align: left; }
  .menu-entry:hover, .sort-submenu button:hover, .sort-submenu button.selected { color: white; background: #3f3f46; }
  .menu-entry:disabled { cursor: wait; opacity: .65; }
  .menu-divider { height: 1px; margin: 5px 4px; background: #3f3f46; }
  .switch { width: 34px; height: 19px; flex-shrink: 0; padding: 2px; border-radius: 10px; background: #52525b; transition: background .15s ease; }
  .switch i { display: block; width: 15px; height: 15px; border-radius: 50%; background: #e4e4e7; transition: transform .15s ease; }
  .switch.enabled { background: #dc2626; }
  .switch.enabled i { transform: translateX(15px); }
  .submenu-row { position: relative; }
  .sort-submenu { display: none; position: absolute; top: -6px; left: calc(100% + 6px); width: 190px; padding: 6px; border: 1px solid #3f3f46; border-radius: 8px; background: #202023; box-shadow: 0 14px 35px rgb(0 0 0 / 45%); }
  .submenu-row:hover .sort-submenu, .submenu-row:focus-within .sort-submenu { display: block; }
  kbd { padding: 2px 6px; border: 1px solid #52525b; border-radius: 4px; color: #a1a1aa; background: #18181b; font-size: 11px; }
  .paths { width: 300px; min-width: 0; overflow: hidden; color: #a1a1aa; font-size: 12px; }
  .paths form { display: flex; gap: 4px; }
  .paths input { min-width: 0; flex: 1; color: #e4e4e7; background: #27272a; border: 1px solid #52525b; border-radius: 4px; padding: 4px; }
  .paths form button { color: white; background: #3f3f46; border: 0; border-radius: 4px; cursor: pointer; }
  .paths small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .paths small { margin-top: 3px; color: #52525b; }
  .assistant-button { height: 38px; display: flex; align-items: center; gap: 7px; padding: 0 13px; border: 1px solid #52525b; border-radius: 6px; color: #e4e4e7; background: #27272a; cursor: pointer; }
  .assistant-button span { color: #ef4444; }
  .assistant-button:hover, .assistant-button.active { border-color: #ef4444; color: white; background: #3f1d22; }
  .remote-button { height: 38px; display: flex; align-items: center; gap: 7px; padding: 0 12px; border: 1px solid #3f3f46; border-radius: 6px; color: #a1a1aa; background: #202023; cursor: pointer; font-size: 11px; font-weight: 800; letter-spacing: .8px; }
  .remote-button:hover, .remote-button.paired { border-color: #71717a; color: #e4e4e7; }
  .remote-button.connected { border-color: #166534; color: #bbf7d0; background: #12261a; }
  .remote-dot { width: 8px; height: 8px; border-radius: 50%; background: #52525b; }
  .remote-button.paired .remote-dot { background: #f59e0b; }
  .remote-button.connected .remote-dot { background: #4ade80; box-shadow: 0 0 10px #22c55e; }
  .rescan { height: 38px; padding: 0 16px; border: 0; border-radius: 6px; color: white; background: #dc2626; cursor: pointer; }
  .rescan:disabled { cursor: wait; opacity: .6; }
</style>
