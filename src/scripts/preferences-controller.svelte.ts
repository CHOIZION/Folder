import { getAppPreferences, setMinimizeToTray } from "./heart-api";

export class PreferencesController {
  minimizeToTray = $state(false);
  saving = $state(false);

  constructor(private readonly onError: (error: unknown) => void) {}

  initialize = async (): Promise<void> => {
    try {
      const preferences = await getAppPreferences();
      this.minimizeToTray = preferences.minimizeToTray;
    } catch (error) {
      this.onError(error);
    }
  };

  setMinimizeToTray = async (enabled: boolean): Promise<void> => {
    if (this.saving || enabled === this.minimizeToTray) return;
    this.saving = true;
    try {
      const preferences = await setMinimizeToTray(enabled);
      this.minimizeToTray = preferences.minimizeToTray;
    } catch (error) {
      this.onError(error);
    } finally {
      this.saving = false;
    }
  };
}
