<script lang="ts">
  import type { RemoteHostStatus } from "../scripts/types";

  let {
    status,
    loading,
    onRefresh,
    onReset,
    onClose
  }: {
    status: RemoteHostStatus | null;
    loading: boolean;
    onRefresh: () => void;
    onReset: () => void;
    onClose: () => void;
  } = $props();

  let copied = $state<"address" | "tailscale" | "code" | null>(null);

  async function copy(value: string, kind: "address" | "tailscale" | "code"): Promise<void> {
    try {
      await navigator.clipboard.writeText(value);
      copied = kind;
      window.setTimeout(() => {
        if (copied === kind) copied = null;
      }, 1200);
    } catch {
      copied = null;
    }
  }
</script>

<div class="backdrop" role="presentation" onclick={(event) => event.currentTarget === event.target && onClose()}>
  <div class="panel" role="dialog" aria-modal="true" aria-label="HEART Remote 연결">
    <header>
      <div>
        <span class="eyebrow">ANDROID COMPANION</span>
        <h2>HEART Remote</h2>
      </div>
      <button class="close" aria-label="닫기" onclick={onClose}>×</button>
    </header>

    {#if status?.errorMessage}
      <div class="warning">
        <strong>원격 서버를 준비하지 못했습니다.</strong>
        <span>{status.errorMessage}</span>
      </div>
    {:else}
      <div class:online={status?.connected} class="connection-state">
        <span class="signal"><i></i></span>
        <div>
          <strong>{status?.connected ? "Android 연결됨" : status?.pairedDevice ? "페어링됨 · 응답 대기 중" : "연결 대기 중"}</strong>
          <small>{status?.pairedDevice ?? "아직 등록된 휴대폰이 없습니다."}</small>
        </div>
      </div>

      {#if status?.h264Encoder}
        <div class="codec-state">
          <strong>H.264 {status.h264HardwareAccelerated ? "하드웨어 가속" : "소프트웨어 폴백"}</strong>
          <span>{status.h264Width}×{status.h264Height} · {status.h264Encoder}</span>
        </div>
      {:else if status?.h264Error}
        <div class="codec-state codec-error">
          <strong>H.264 호환 스트림으로 전환됨</strong>
          <span>{status.h264Error}</span>
        </div>
      {/if}

      <div class="pair-grid">
        <div class="pair-card">
          <div class="field-label">노트북 주소</div>
          <button onclick={() => status && copy(status.connectionUrl, "address")} disabled={!status}>
            <span>{status?.connectionUrl ?? "확인 중..."}</span>
            <small>{copied === "address" ? "복사됨" : "복사"}</small>
          </button>
        </div>
        {#if status?.tailscaleConnectionUrl}
          <div class="pair-card tailscale-card">
            <div class="field-label">Tailscale 주소 · 외부 연결</div>
            <button onclick={() => status && copy(status.tailscaleConnectionUrl!, "tailscale")}>
              <span>{status.tailscaleConnectionUrl}</span>
              <small>{copied === "tailscale" ? "복사됨" : "복사"}</small>
            </button>
          </div>
        {/if}
        <div class="pair-card code-card">
          <div class="field-label">6자리 페어링 코드</div>
          <button onclick={() => status && copy(status.pairingCode, "code")} disabled={!status}>
            <span>{status?.pairingCode ?? "------"}</span>
            <small>{copied === "code" ? "복사됨" : "복사"}</small>
          </button>
        </div>
      </div>

      {#if status?.tailscaleConnectionUrl}
        <ol>
          <li>같은 Wi-Fi라면 Android에서 <b>HEART 자동 찾기</b>를 누릅니다.</li>
          <li>다른 네트워크라면 두 기기에서 Tailscale에 로그인한 뒤, 위 <b>Tailscale 주소</b>를 Android 주소 칸에 붙여 넣습니다.</li>
          <li>위 6자리 코드를 입력하면 연결 확인이 완료됩니다.</li>
        </ol>
      {:else}
        <ol>
          <li>노트북과 Android를 같은 Wi-Fi에 연결합니다.</li>
          <li>Android에서 HEART Remote를 열고 <b>HEART 자동 찾기</b>를 누릅니다.</li>
          <li>위 6자리 코드를 입력하면 연결 확인이 완료됩니다.</li>
        </ol>
      {/if}

      <p class="firewall">Windows 방화벽 메시지가 나타나면 <b>개인 네트워크</b>만 허용해 주세요.</p>
    {/if}

    <footer>
      <button class="secondary" onclick={onRefresh} disabled={loading}>{loading ? "확인 중..." : "상태 새로고침"}</button>
      <button class="danger" onclick={onReset} disabled={loading || !!status?.errorMessage}>휴대폰 연결 초기화</button>
    </footer>
  </div>
</div>

<style>
  .backdrop { position: fixed; z-index: 300; inset: 0; display: grid; place-items: center; padding: 24px; background: rgb(0 0 0 / 66%); backdrop-filter: blur(5px); }
  .panel { width: min(610px, 100%); overflow: hidden; border: 1px solid #3f3f46; border-radius: 18px; color: #f4f4f5; background: linear-gradient(145deg, #202026, #131316); box-shadow: 0 28px 90px rgb(0 0 0 / 70%); }
  header { display: flex; align-items: flex-start; justify-content: space-between; padding: 25px 27px 18px; border-bottom: 1px solid #303036; }
  .eyebrow { color: #ef4444; font-size: 10px; font-weight: 800; letter-spacing: 2px; }
  h2 { margin: 5px 0 0; font-size: 26px; letter-spacing: .5px; }
  .close { width: 36px; height: 36px; border: 0; border-radius: 9px; color: #a1a1aa; background: #29292f; cursor: pointer; font-size: 25px; line-height: 1; }
  .close:hover { color: white; background: #3f3f46; }
  .connection-state { display: flex; align-items: center; gap: 14px; margin: 22px 27px 17px; padding: 15px 17px; border: 1px solid #3f3f46; border-radius: 12px; background: #202024; }
  .connection-state.online { border-color: #166534; background: #12261a; }
  .signal { width: 34px; height: 34px; display: grid; place-items: center; border-radius: 50%; background: #3f3f46; }
  .signal i { width: 10px; height: 10px; border-radius: 50%; background: #71717a; box-shadow: 0 0 0 5px rgb(113 113 122 / 15%); }
  .online .signal { background: #14532d; }
  .online .signal i { background: #4ade80; box-shadow: 0 0 0 5px rgb(74 222 128 / 13%), 0 0 18px #22c55e; }
  .connection-state div { display: flex; flex-direction: column; gap: 4px; }
  .connection-state strong { font-size: 14px; }
  .connection-state small { color: #a1a1aa; font-size: 12px; }
  .codec-state { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin: -8px 27px 17px; padding: 10px 13px; border: 1px solid #14532d; border-radius: 9px; color: #bbf7d0; background: #10251a; font-size: 11px; }
  .codec-state span { min-width: 0; overflow: hidden; color: #86efac; text-overflow: ellipsis; white-space: nowrap; }
  .codec-error { border-color: #7f1d1d; color: #fecaca; background: #35171a; }
  .codec-error span { color: #fca5a5; }
  .pair-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; padding: 0 27px; }
  .tailscale-card { grid-column: 1 / -1; }
  .pair-card { min-width: 0; }
  .field-label { display: block; margin: 0 0 7px 2px; color: #71717a; font-size: 11px; font-weight: 700; }
  .pair-card button { width: 100%; height: 58px; display: flex; align-items: center; justify-content: space-between; gap: 9px; padding: 0 14px; border: 1px solid #3f3f46; border-radius: 10px; color: #e4e4e7; background: #27272a; cursor: pointer; }
  .pair-card button:hover { border-color: #71717a; }
  .pair-card span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: Consolas, monospace; font-size: 13px; }
  .pair-card small { color: #71717a; font-size: 10px; }
  .code-card span { color: #fca5a5; font-size: 25px; font-weight: 800; letter-spacing: 4px; }
  ol { margin: 22px 27px 12px; padding: 18px 22px 18px 42px; border-radius: 12px; color: #d4d4d8; background: #18181c; font-size: 13px; line-height: 1.8; }
  ol b, .firewall b { color: white; }
  .firewall { margin: 0 27px; color: #a1a1aa; font-size: 11px; }
  footer { display: flex; justify-content: flex-end; gap: 9px; padding: 22px 27px 25px; }
  footer button { height: 38px; padding: 0 14px; border-radius: 7px; cursor: pointer; }
  footer button:disabled { cursor: wait; opacity: .55; }
  .secondary { border: 1px solid #52525b; color: #e4e4e7; background: #27272a; }
  .danger { border: 1px solid #7f1d1d; color: #fecaca; background: #3b171b; }
  .warning { display: flex; flex-direction: column; gap: 7px; margin: 24px 27px 4px; padding: 16px; border: 1px solid #991b1b; border-radius: 10px; color: #fecaca; background: #451a1a; font-size: 12px; }
  @media (max-width: 560px) { .pair-grid { grid-template-columns: 1fr; } }
</style>
