<script lang="ts">
  import { tick } from "svelte";
  import {
    applyTagSuggestion,
    getTagSuggestions,
    type TagSuggestion,
    type TagSuggestionMatch
  } from "../scripts/search";
  import type { TagCatalogEntry } from "../scripts/tags";

  let {
    value,
    catalog,
    onInput,
    placeholder = "제목, 경로, 태그, 메모 검색"
  }: {
    value: string;
    catalog: TagCatalogEntry[];
    onInput: (value: string) => void;
    placeholder?: string;
  } = $props();

  let inputElement: HTMLInputElement | null = null;
  let suggestionsVisible = $state(false);
  let activeIndex = $state(0);
  let suggestions = $derived(getTagSuggestions(catalog, value));
  let dropdownOpen = $derived(suggestionsVisible && suggestions.length > 0);

  const matchLabels: Record<TagSuggestionMatch, string> = {
    exact: "정확",
    prefix: "자동완성",
    contains: "포함",
    related: "연관"
  };

  function handleInput(event: Event): void {
    activeIndex = 0;
    suggestionsVisible = true;
    onInput((event.currentTarget as HTMLInputElement).value);
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      suggestionsVisible = false;
      return;
    }
    if (!dropdownOpen) return;

    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const direction = event.key === "ArrowDown" ? 1 : -1;
      activeIndex = (activeIndex + direction + suggestions.length) % suggestions.length;
      return;
    }
    if (event.key === "Enter" || event.key === "Tab") {
      event.preventDefault();
      chooseSuggestion(suggestions[activeIndex] ?? suggestions[0]);
    }
  }

  function chooseSuggestion(suggestion: TagSuggestion): void {
    onInput(applyTagSuggestion(value, suggestion.tag));
    suggestionsVisible = false;
    void tick().then(() => inputElement?.focus());
  }
</script>

<div class="tag-search">
  <input
    bind:this={inputElement}
    data-heart-search
    type="search"
    role="combobox"
    autocomplete="off"
    aria-label="라이브러리 검색"
    aria-autocomplete="list"
    aria-expanded={dropdownOpen}
    aria-controls="heart-tag-suggestions"
    aria-activedescendant={dropdownOpen ? `heart-tag-${activeIndex}` : undefined}
    {placeholder}
    {value}
    oninput={handleInput}
    onkeydown={handleKeydown}
    onfocus={() => suggestionsVisible = true}
    onblur={() => suggestionsVisible = false}
  />

  {#if dropdownOpen}
    <div id="heart-tag-suggestions" class="suggestions" role="listbox" aria-label="태그 자동완성 및 연관 태그">
      {#each suggestions as suggestion, index (suggestion.normalized)}
        <button
          id={`heart-tag-${index}`}
          type="button"
          role="option"
          aria-selected={index === activeIndex}
          class:active={index === activeIndex}
          onmousedown={(event) => event.preventDefault()}
          onmouseenter={() => activeIndex = index}
          onclick={() => chooseSuggestion(suggestion)}
        >
          <span class="kind {suggestion.kind}">{suggestion.kind === "topic" ? "◎" : suggestion.kind === "genre" ? "▣" : "◇"}</span>
          <span class="tag">{suggestion.tag}</span>
          <small>{matchLabels[suggestion.match]} · {suggestion.count}개</small>
        </button>
      {/each}
      <p><kbd>↑</kbd><kbd>↓</kbd> 이동 · <kbd>Enter</kbd> 선택 · 쉼표로 여러 태그 AND 검색</p>
    </div>
  {/if}
</div>

<style>
  .tag-search { position: relative; flex: 1; min-width: 120px; }
  input { width: 100%; height: 38px; padding: 0 14px; border: 1px solid #3f3f46; border-radius: 6px; outline: none; color: #f4f4f5; background: #27272a; }
  input:focus { border-color: #ef4444; box-shadow: 0 0 0 3px rgb(239 68 68 / 10%); }
  .suggestions { position: absolute; z-index: 80; top: calc(100% + 7px); right: 0; left: 0; min-width: 330px; overflow: hidden; border: 1px solid #3f3f46; border-radius: 9px; background: #202023; box-shadow: 0 16px 40px rgb(0 0 0 / 50%); }
  .suggestions button { width: 100%; min-height: 38px; display: grid; grid-template-columns: 22px minmax(0, 1fr) auto; align-items: center; gap: 7px; padding: 5px 10px; border: 0; color: #e4e4e7; background: transparent; text-align: left; cursor: pointer; }
  .suggestions button:hover, .suggestions button.active { color: #fff; background: #3f3f46; }
  .kind { font-weight: 900; text-align: center; }
  .kind.topic { color: #f9a8d4; }
  .kind.genre { color: #93c5fd; }
  .kind.etc { color: #a1a1aa; }
  .tag { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; font-weight: 700; }
  small { color: #71717a; font-size: 10px; white-space: nowrap; }
  .suggestions p { margin: 0; padding: 7px 10px; border-top: 1px solid #303036; color: #71717a; font-size: 10px; }
  kbd { margin-right: 2px; padding: 1px 4px; border: 1px solid #52525b; border-radius: 3px; color: #a1a1aa; background: #18181b; font-size: 9px; }
</style>
