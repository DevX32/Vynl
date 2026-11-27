import { getCurrentTrack, updatePlaying } from "@state/now-playing.svelte";
import { setPlayError } from "@state/player.svelte";
import { t } from "@lib/i18n";
import { friendlyError } from "@lib/errors";
import { isPlayInterrupt } from "./playback-utils";

export const MAX_CONSECUTIVE_PLAY_FAILURES = 3;
export const retryState = {
  pending: null as number | null,
  failures: 0,
};

let deps = {
  getPauseIntent: () => 0,
  next: () => {},
  updateMediaSession: () => {},
};
export function initRetry(d: typeof deps): void { deps = d; }

export function clearRetry(): void {
  if (retryState.pending !== null) {
    clearTimeout(retryState.pending);
    retryState.pending = null;
  }
}

export function scheduleRetry(failedId: string | null): void {
  clearRetry();
  retryState.pending = window.setTimeout(() => {
    retryState.pending = null;
    if (deps.getPauseIntent() > 0) return;
    if ((getCurrentTrack()?.id ?? null) !== failedId) return;
    if (retryState.failures >= MAX_CONSECUTIVE_PLAY_FAILURES) {
      updatePlaying(false);
      deps.updateMediaSession();
      return;
    }
    setPlayError(null);
    deps.next();
  }, 1600);
}

export function handlePlayReject(err: unknown): void {
  if (isPlayInterrupt(err)) return;
  console.error("[vynl] native play failed:", err);
  updatePlaying(false);
  deps.updateMediaSession();
  retryState.failures += 1;
  if (retryState.failures >= MAX_CONSECUTIVE_PLAY_FAILURES) {
    setPlayError(t("error.couldNotPlay"));
    return;
  }
  setPlayError(friendlyError(err, t("error.couldNotPlay")));
  const failedId = getCurrentTrack()?.id ?? null;
  scheduleRetry(failedId);
}
