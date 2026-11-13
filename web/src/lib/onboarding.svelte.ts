import { getCurrentSettings, patchSettings } from "@state/settings.svelte";

const STORAGE_KEY = "vynl.onboarded";

export function wasOnboarded(): boolean {
  try {
    if (localStorage.getItem(STORAGE_KEY) === "1") return true;
    return getCurrentSettings().displayName.trim().length > 0;
  } catch {
    return true;
  }
}

export async function completeOnboarding(name: string): Promise<void> {
  const trimmed = name.trim().slice(0, 32);
  try {
    await patchSettings({ displayName: trimmed });
  } catch {
    // keep going — the localStorage flag is what gates the screen
  }
  try {
    localStorage.setItem(STORAGE_KEY, "1");
  } catch {
    // localStorage unavailable — private mode, etc.
  }
}
