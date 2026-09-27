<script lang="ts">
  import type { HeartMode, SessionPlan } from "../scripts/assistant";

  let {
    mode,
    listening,
    supported,
    transcript,
    response,
    plans,
    selectedPlanId,
    currentStepIndex,
    remainingSeconds,
    running,
    onMode,
    onSelectPlan,
    onToggleListening,
    onStartPlan,
    onNext,
    onPrevious,
    onStop,
    onClose
  }: {
    mode: HeartMode;
    listening: boolean;
    supported: boolean;
    transcript: string;
    response: string;
    plans: SessionPlan[];
    selectedPlanId: string | null;
    currentStepIndex: number;
    remainingSeconds: number;
    running: boolean;
    onMode: (mode: HeartMode) => void;
    onSelectPlan: (planId: string) => void;
    onToggleListening: () => void;
    onStartPlan: () => void;
    onNext: () => void;
    onPrevious: () => void;
    onStop: () => void;
    onClose: () => void;
  } = $props();

  let selectedPlan = $derived(
    plans.find((candidate) => candidate.id === selectedPlanId) ?? plans[0] ?? null
  );

  function clock(seconds: number): string {
    const safe = Math.max(0, seconds);
    return `${String(Math.floor(safe / 60)).padStart(2, "0")}:${String(safe % 60).padStart(2, "0")}`;
  }
</script>

<aside class="assistant">
  <header>
    <div>
      <small>HEART ASSISTANT</small>
      <h2>{mode === "jarvis" ? "자비스 모드" : mode === "handsfree" ? "핸즈프리 모드" : "기본 모드"}</h2>
    </div>
    <button class="close" onclick={onClose} aria-label="닫기">×</button>
  </header>

  <div class="modes">
    <button class:active={mode === "basic"} onclick={() => onMode("basic")}>기본</button>
    <button class:active={mode === "handsfree"} onclick={() => onMode("handsfree")}>핸즈프리</button>
    <button class:active={mode === "jarvis"} onclick={() => onMode("jarvis")}>자비스</button>
  </div>

  {#if mode !== "basic"}
    <button class:listening class="mic" onclick={onToggleListening} disabled={!supported}>
      <span>{listening ? "●" : "◉"}</span>
      {supported ? (listening ? "듣는 중 · 눌러서 중지" : "음성 인식 시작") : "이 환경은 음성 인식을 지원하지 않음"}
    </button>
    <div class="speech">
      <small>인식</small>
      <p>{transcript || "“자비스, 도입 포함 40분 세팅 부탁해”"}</p>
      {#if response}<strong>{response}</strong>{/if}
    </div>
  {/if}

  {#if mode === "jarvis" && plans.length > 0}
    <section class="plan">
      <div class="choices">
        {#each plans as candidate (candidate.id)}
          <button
            class:selected={candidate.id === selectedPlanId}
            onclick={() => onSelectPlan(candidate.id)}
          >
            <strong>{candidate.name}</strong>
            <small>{candidate.steps.length}개 작품 · {candidate.description}</small>
          </button>
        {/each}
      </div>

      {#if selectedPlan}
      <div class="plan-title">
        <div>
          <small>{selectedPlan.requestedMinutes}분 코스</small>
          <h3>{selectedPlan.warmup === "none" ? selectedPlan.name : `도입 포함 ${selectedPlan.name}`}</h3>
        </div>
        {#if running}<span class="timer">{clock(remainingSeconds)}</span>{/if}
      </div>

      <div class="steps">
        {#each selectedPlan.steps as step, index (step.id)}
          <button class:current={index === currentStepIndex} onclick={() => {}}>
            <span class="number">{index + 1}</span>
            <span class="step-copy">
              <strong>{step.item.title}</strong>
              <small>{step.durationMinutes}분 · {step.reason}</small>
            </span>
          </button>
        {/each}
      </div>

      <div class="controls">
          {#if !running}
            <button class="primary" onclick={onStartPlan}>이 코스로 시작</button>
          {:else}
            <button onclick={onPrevious}>이전</button>
            <button class="primary" onclick={onNext}>종료 후 다음</button>
            <button class="danger" onclick={onStop}>세션 중지</button>
          {/if}
        </div>
      {/if}
    </section>
  {:else if mode === "jarvis"}
    <div class="empty-plan">
      <strong>말 한마디로 코스를 만드세요.</strong>
      <span>“자비스, 30분 세팅”</span>
      <span>“자비스, 20분 도입 코스”</span>
    </div>
  {:else if mode === "handsfree"}
    <div class="commands">
      <strong>음성 명령</strong>
      <span>만화 · 이미지 · 게임 · 렌파이 · 쯔꾸르 · 영상 · 문서</span>
      <span>아래 · 위 · 랜덤</span>
      <span>다음 · 이전 · 실행 · 열어 · 재생</span>
      <span>즐겨찾기 · 좋아요 · 중지 · 정지 · 종료</span>
    </div>
  {/if}
</aside>

<style>
  .assistant { width: 390px; min-width: 390px; height: 100%; padding: 18px; overflow-y: auto; border-left: 1px solid #3f3f46; background: linear-gradient(180deg, #18181b, #111114); }
  header, .plan-title { display: flex; align-items: center; justify-content: space-between; gap: 14px; }
  header small, .plan-title small, .speech small { color: #ef4444; font-size: 10px; letter-spacing: 1.4px; }
  h2, h3, p { margin: 0; }
  h2 { margin-top: 4px; font-size: 20px; }
  h3 { margin-top: 3px; font-size: 16px; }
  button { border: 1px solid #3f3f46; border-radius: 7px; color: #e4e4e7; background: #27272a; cursor: pointer; }
  button:hover { border-color: #71717a; background: #323238; }
  .close { width: 34px; height: 34px; font-size: 21px; }
  .modes { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; margin: 20px 0 12px; }
  .modes button { height: 36px; }
  .modes button.active { border-color: #ef4444; color: white; background: #991b1b; }
  .mic { width: 100%; min-height: 46px; display: flex; align-items: center; justify-content: center; gap: 9px; }
  .mic span { color: #ef4444; }
  .mic.listening { border-color: #ef4444; background: #451a1a; box-shadow: 0 0 0 4px rgb(239 68 68 / 10%); }
  .speech, .commands, .empty-plan { margin-top: 12px; padding: 13px; border: 1px solid #27272a; border-radius: 8px; background: #151518; }
  .speech p { margin: 7px 0; color: #d4d4d8; line-height: 1.5; }
  .speech strong { color: #fca5a5; font-size: 12px; }
  .plan { margin-top: 20px; }
  .choices { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; margin-bottom: 15px; }
  .choices button { min-width: 0; min-height: 72px; display: grid; align-content: center; gap: 5px; padding: 8px; text-align: left; }
  .choices button.selected { border-color: #ef4444; background: #3f1d22; }
  .choices strong, .choices small { overflow: hidden; text-overflow: ellipsis; }
  .choices small { color: #a1a1aa; font-size: 9px; line-height: 1.35; }
  .timer { color: #facc15; font-size: 23px; font-variant-numeric: tabular-nums; }
  .steps { display: grid; gap: 7px; margin-top: 13px; }
  .steps button { width: 100%; min-height: 56px; display: flex; align-items: center; gap: 10px; padding: 8px 10px; text-align: left; }
  .steps button.current { border-color: #ef4444; background: #3f1d22; }
  .number { width: 26px; height: 26px; display: grid; place-items: center; flex: 0 0 auto; border-radius: 50%; color: #a1a1aa; background: #18181b; font-size: 11px; }
  .step-copy { min-width: 0; display: grid; gap: 4px; }
  .step-copy strong, .step-copy small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .step-copy small { color: #a1a1aa; font-size: 10px; }
  .controls { display: grid; grid-template-columns: repeat(3, 1fr); gap: 7px; margin-top: 14px; }
  .controls button { min-height: 40px; }
  .controls .primary { border-color: #dc2626; background: #dc2626; }
  .controls .danger { color: #fca5a5; }
  .empty-plan, .commands { display: grid; gap: 7px; color: #a1a1aa; font-size: 12px; line-height: 1.5; }
  .empty-plan strong, .commands strong { color: #e4e4e7; font-size: 13px; }
</style>
