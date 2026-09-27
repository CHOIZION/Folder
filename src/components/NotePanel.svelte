<script lang="ts">
  import { onMount } from "svelte";

  const STORAGE_KEY = "heart.personal.memo";
  let note = $state("");
  let savedLabel = $state("자동 저장");
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(() => {
    note = localStorage.getItem(STORAGE_KEY) ?? "";
  });

  function saveNote(): void {
    if (saveTimer) clearTimeout(saveTimer);
    savedLabel = "저장 중...";
    saveTimer = setTimeout(() => {
      localStorage.setItem(STORAGE_KEY, note);
      const now = new Date();
      savedLabel = `자동 저장됨 · ${now.toLocaleTimeString("ko-KR", { hour: "2-digit", minute: "2-digit" })}`;
    }, 250);
  }
</script>

<section class="note-panel">
  <header>
    <div>
      <h1>메모장</h1>
      <p>HEART 안에서 자유롭게 메모하실 수 있습니다.</p>
    </div>
    <span>{savedLabel}</span>
  </header>

  <textarea
    bind:value={note}
    oninput={saveNote}
    spellcheck="false"
    placeholder="메모를 입력하세요..."
    aria-label="HEART 메모장"
  ></textarea>
</section>

<style>
  .note-panel { min-width: 0; flex: 1; display: flex; flex-direction: column; padding: 28px 32px 32px; background: #101014; }
  header { display: flex; align-items: flex-end; justify-content: space-between; gap: 24px; margin-bottom: 18px; }
  h1 { margin: 0 0 6px; font-size: 27px; font-weight: 800; }
  p { margin: 0; color: #8b8b95; font-size: 13px; }
  header span { color: #71717a; font-size: 12px; white-space: nowrap; }
  textarea { width: 100%; min-height: 0; flex: 1; resize: none; border: 1px solid #34343b; border-radius: 12px; outline: none; padding: 22px 24px; color: #f4f4f5; background: #18181d; font-size: 16px; line-height: 1.75; box-shadow: inset 0 1px 0 rgba(255,255,255,.025); }
  textarea:focus { border-color: #666672; background: #1b1b20; }
  textarea::placeholder { color: #55555f; }
</style>
