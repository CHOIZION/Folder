import { getRemoteHostStatus, resetRemotePairing } from "./heart-api";
import type { RemoteHostStatus } from "./types";

const REFRESH_INTERVAL_MS = 3000;

export class RemoteController {
  open = $state(false);
  loading = $state(false);
  status = $state<RemoteHostStatus | null>(null);

  private timer: ReturnType<typeof setInterval> | null = null;
  private readonly reportError: (error: unknown) => void;

  constructor(reportError: (error: unknown) => void) {
    this.reportError = reportError;
  }

  start(): void {
    void this.refresh();
    this.timer = setInterval(
      () => void this.refresh(false),
      REFRESH_INTERVAL_MS,
    );
  }

  stop(): void {
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
  }

  show(): void {
    this.open = true;
    void this.refresh();
  }

  refresh = async (showLoading = true): Promise<void> => {
    if (showLoading) this.loading = true;
    try {
      this.status = await getRemoteHostStatus();
    } catch (error) {
      if (this.open) this.reportError(error);
    } finally {
      if (showLoading) this.loading = false;
    }
  };

  reset = async (): Promise<void> => {
    if (!window.confirm("등록된 휴대폰 연결을 끊고 새 페어링 코드를 만들까요?"))
      return;
    this.loading = true;
    try {
      this.status = await resetRemotePairing();
    } catch (error) {
      this.reportError(error);
    } finally {
      this.loading = false;
    }
  };
}
