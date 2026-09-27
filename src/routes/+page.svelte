<script lang="ts">
  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";

  import AssistantPanel from "../components/AssistantPanel.svelte";
  import DetailPanel from "../components/DetailPanel.svelte";
  import MediaGrid from "../components/MediaGrid.svelte";
  import NotePanel from "../components/NotePanel.svelte";
  import PulseWheel from "../components/PulseWheel.svelte";
  import RemotePanel from "../components/RemotePanel.svelte";
  import Sidebar from "../components/Sidebar.svelte";
  import StatisticsPanel from "../components/StatisticsPanel.svelte";
  import TopBar from "../components/TopBar.svelte";

  import { AssistantController } from "../scripts/assistant-controller.svelte";
  import { LibraryController } from "../scripts/library-controller.svelte";
  import { PulseController } from "../scripts/pulse-controller.svelte";
  import { PreferencesController } from "../scripts/preferences-controller.svelte";
  import { RemoteController } from "../scripts/remote-controller.svelte";
  import type { LibraryUpdatedEvent } from "../scripts/types";

  const library = new LibraryController();
  const assistant = new AssistantController(library);
  const pulse = new PulseController(library);
  const remote = new RemoteController((error) => library.setError(error));
  const preferences = new PreferencesController((error) => library.setError(error));

  onMount(() => {
    const unlisteners: UnlistenFn[] = [];
    let disposed = false;
    const subscribe = async (): Promise<void> => {
      const listeners = await Promise.all([
        listen<LibraryUpdatedEvent>("library-updated", (event) => {
          void library.reloadFromWatcher(event.payload.rootPath);
        }),
        listen<string>("library-watch-error", (event) => {
          library.setError(event.payload);
        })
      ]);
      if (disposed) listeners.forEach((unlisten) => unlisten());
      else unlisteners.push(...listeners);
    };
    void subscribe();
    void library.initialize();
    void preferences.initialize();
    remote.start();
    assistant.start();

    const handleKeydown = (event: KeyboardEvent): void => {
      const target = event.target as HTMLElement | null;
      if (target?.matches("input, textarea, select, [contenteditable=true]")) return;
      if (event.ctrlKey || event.metaKey || event.altKey) return;

      if (event.key.toLowerCase() === "r") {
        event.preventDefault();
        library.selectRandomItem();
      } else if (event.key === "Enter" && library.selectedItem) {
        event.preventDefault();
        void library.openItem(library.selectedItem);
      }
    };
    const handleMiddleMouse = (event: MouseEvent): void => pulse.handleMiddleMouse(event);
    const preventMiddleDefault = (event: MouseEvent): void => {
      if (event.button === 1) event.preventDefault();
    };

    window.addEventListener("keydown", handleKeydown);
    window.addEventListener("mousedown", handleMiddleMouse, true);
    window.addEventListener("auxclick", preventMiddleDefault, true);
    return () => {
      window.removeEventListener("keydown", handleKeydown);
      window.removeEventListener("mousedown", handleMiddleMouse, true);
      window.removeEventListener("auxclick", preventMiddleDefault, true);
      assistant.dispose();
      remote.stop();
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  });
</script>

<svelte:head><title>HEART</title></svelte:head>

<div class="app">
  <TopBar
    searchText={library.searchText}
    tagCatalog={library.tagCatalog}
    rootPath={library.rootPath}
    databasePath={library.databasePath}
    scanning={library.loading}
    sortMode={library.sortMode}
    onSearch={(value) => library.searchText = value}
    onRescan={library.rescan}
    onChooseRoot={library.chooseRoot}
    onSort={(mode) => library.sortMode = mode}
    onRandom={() => library.selectRandomItem()}
    onStatistics={() => library.statisticsOpen = true}
    remoteConnected={remote.status?.connected ?? false}
    remotePaired={!!remote.status?.pairedDevice}
    onRemote={() => remote.show()}
    assistantActive={assistant.open}
    onAssistant={() => assistant.open = !assistant.open}
    minimizeToTray={preferences.minimizeToTray}
    savingPreferences={preferences.saving}
    onMinimizeToTray={(enabled) => void preferences.setMinimizeToTray(enabled)}
  />

  {#if library.errorMessage}
    <div class="error-message">
      <strong>오류</strong><span>{library.errorMessage}</span>
      <button onclick={() => library.errorMessage = ""}>닫기</button>
    </div>
  {/if}

  <div class="workspace">
    <Sidebar
      categories={library.snapshot?.categories ?? []}
      userCategories={library.snapshot?.userCategories ?? []}
      selectedCategoryId={library.selectedCategoryId}
      selectedRating={library.selectedRating}
      unclassifiedCount={library.counts.unclassified}
      favoriteCount={library.counts.favorites}
      recentCount={library.counts.recent}
      ratingCounts={library.counts.ratingCounts}
      totalCount={library.snapshot?.totalItems ?? 0}
      onSelect={(id) => library.selectCategory(id)}
      onSelectRating={(rating) => library.selectRating(rating)}
      onCreateUserCategory={library.addUserCategory}
      onDeleteUserCategory={library.removeUserCategory}
      memoActive={library.activeView === "memo"}
      onOpenMemo={() => library.showMemo()}
    />

    {#if library.activeView === "memo"}
      <NotePanel />
    {:else}
      <MediaGrid
        title={`${library.currentTitle}${library.selectedRating ? ` · ${library.selectedRating}점` : ""}`}
        items={library.visibleItems}
        sections={library.visibleSections}
        selectedItemId={library.selectedItemPath}
        loading={library.loading}
        onSelect={(item) => library.selectItem(item)}
        onOpen={library.openItem}
        onOpenVideo={library.openVideo}
      />

      {#if library.detailPanelOpen && library.selectedItem}
        <DetailPanel
          item={library.selectedItem}
          userCategories={library.snapshot?.userCategories ?? []}
          saving={library.saving}
          onClose={() => library.detailPanelOpen = false}
          onSave={library.saveMetadata}
          onOpen={library.openItem}
          onOpenFolder={library.openItemFolder}
          onSetCategories={library.saveCategories}
        />
      {/if}
    {/if}

    {#if assistant.open}
      <AssistantPanel
        mode={assistant.mode}
        listening={assistant.listening}
        supported={assistant.supported}
        transcript={assistant.transcript}
        response={assistant.response}
        plans={assistant.plans}
        selectedPlanId={assistant.selectedPlanId}
        currentStepIndex={assistant.stepIndex}
        remainingSeconds={assistant.remainingSeconds}
        running={assistant.running}
        onMode={assistant.setMode}
        onSelectPlan={assistant.selectPlan}
        onToggleListening={assistant.toggleListening}
        onStartPlan={assistant.startSession}
        onNext={assistant.nextStep}
        onPrevious={assistant.previousStep}
        onStop={assistant.stopSession}
        onClose={() => assistant.open = false}
      />
    {/if}
  </div>

  {#if library.statisticsOpen}
    <StatisticsPanel items={library.allItems} onClose={() => library.statisticsOpen = false} />
  {/if}

  {#if pulse.open}
    <PulseWheel
      x={pulse.x}
      y={pulse.y}
      totalCount={library.pulse.items.length}
      favoriteCount={library.pulse.favorites}
      recentCount={library.pulse.recent}
      oldestTitle={library.pulse.oldest?.title ?? null}
      lastTitle={library.pulse.latest?.title ?? null}
      onSearch={(query) => pulse.search(query)}
      onOldest={() => pulse.selectItem(library.pulse.oldest)}
      onRecent={() => pulse.showRecent()}
      onRandom={() => pulse.selectRandom()}
      onAll={() => pulse.showAll()}
      onFavorites={() => pulse.showFavorites()}
      onLast={() => pulse.selectItem(library.pulse.latest)}
      onClose={() => pulse.open = false}
    />
  {/if}

  {#if remote.open}
    <RemotePanel
      status={remote.status}
      loading={remote.loading}
      onRefresh={() => void remote.refresh()}
      onReset={() => void remote.reset()}
      onClose={() => remote.open = false}
    />
  {/if}
</div>

<style>
  :global(*) { box-sizing: border-box; }
  :global(html), :global(body) { width: 100%; height: 100%; margin: 0; overflow: hidden; }
  :global(body) { color: #f4f4f5; background: #09090b; font-family: "Noto Sans KR", "Malgun Gothic", Arial, sans-serif; }
  :global(button), :global(input), :global(select), :global(textarea) { font: inherit; }
  .app { width: 100vw; height: 100vh; display: flex; flex-direction: column; }
  .workspace { min-height: 0; flex: 1; display: flex; }
  .error-message { display: flex; align-items: center; gap: 12px; padding: 10px 18px; color: #fecaca; background: #7f1d1d; font-size: 13px; }
  .error-message span { min-width: 0; flex: 1; }
  .error-message button { border: 0; color: white; background: transparent; cursor: pointer; }
</style>
