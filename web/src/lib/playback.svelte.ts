import {
  getCurrentTrack,
  getCurrentTime,
  getIsTrackPlaying,
  setSeekHandler,
} from "@state/now-playing.svelte";
import {
  getCurrentVolume,
  toggleMute,
  onMuteChange,
  onSpeedChange,
  onReplayGainChange,
} from "@state/player.svelte";
import { setSleepPauseHandler } from "@state/sleep-timer.svelte";
import { vynl } from "@lib/vynl";
import { untrack } from "svelte";



import { initSeek, seekRatio, seekFlush, seekToTime, seekOffset, cancelSeek } from "./playback-seek";
export { seekRatio, seekFlush, seekToTime, seekOffset, cancelSeek };
import { initMedia, clearMediaCoverCache, syncMedia, syncMediaNow, getLastMediaSentAt, handleMediaControl, handleMediaPlay, handleMediaPause, handleMediaStop, updateMediaSession } from "./playback-media";
export { clearMediaCoverCache };






onMuteChange(applyVolume);
onSpeedChange(applySpeed);
onReplayGainChange(applyReplayGain);

setSeekHandler(seekToTime);

setSleepPauseHandler(pausePlayback);

if ("mediaSession" in navigator && !IS_MINI) {
  navigator.mediaSession.setActionHandler("play", handleMediaPlay);
  navigator.mediaSession.setActionHandler("pause", handleMediaPause);
  navigator.mediaSession.setActionHandler("nexttrack", next);
  navigator.mediaSession.setActionHandler("previoustrack", prev);
  navigator.mediaSession.setActionHandler("stop", handleMediaStop);
  navigator.mediaSession.setActionHandler("seekto", (details) => {
    if (typeof details.seekTime === "number") seekToTime(details.seekTime);
  });
  navigator.mediaSession.setActionHandler("seekbackward", (details) => {
    seekOffset(-(details.seekOffset ?? 10));
  });
  navigator.mediaSession.setActionHandler("seekforward", (details) => {
    seekOffset(details.seekOffset ?? 10);
  });
}

import { initRetry } from "./playback-retry";
import { initTick } from "./playback-tick";
import { initQueue, nextAutomatic, maybeStartCrossfade } from "./playback-queue";
import { loadAndPlayNative } from "./playback-transport";
import { IS_MINI, ps } from "./playback-state";
import { handleStreamError, handleTick, reconcilePlaybackState, onWindowActive, applyVolume, applySpeed, applyReplayGain, setVolumeAt, playTrack, pausePlayback, toggle, next, prev, clearPlayError, removeQueueTrack, reorderQueueTrack } from "./playback-engine";
export { handleStreamError, handleTick, reconcilePlaybackState, onWindowActive, applyVolume, applySpeed, applyReplayGain, setVolumeAt, playTrack, pausePlayback, toggle, next, prev, clearPlayError, removeQueueTrack, reorderQueueTrack };

initSeek({ getPlayGeneration: () => ps.playGeneration, syncMediaNow });
initMedia({ toggle, next, prev, pausePlayback, seekToTime, seekOffset, setVolumeAt, getCurrentVolume: getCurrentVolume, toggleMute });
initQueue({ loadAndPlayNative });
initTick({ nextAutomatic, maybeStartCrossfade });
initRetry({ getPauseIntent: () => ps.pauseIntent, next, updateMediaSession });

if (typeof window !== "undefined" && !IS_MINI) {
  const disposeMediaEffect = $effect.root(() => {
    $effect(() => {
      const np = getCurrentTrack();
      const playing = getIsTrackPlaying();
      syncMedia(np, playing, untrack(() => getCurrentTime()));
    });
  });
  const mediaTimer = window.setInterval(() => {
    const np = getCurrentTrack();
    if (!np) return;
    if (Date.now() - getLastMediaSentAt() < 1_000) return;
    syncMedia(np, getIsTrackPlaying(), getCurrentTime());
  }, 1_000);
  const unlistenMediaControl = vynl.onMediaControl(handleMediaControl, () => {
    void vynl.mediaControlsReady().catch(() => {});
  });
  import.meta.hot?.dispose(() => {
    disposeMediaEffect();
    window.clearInterval(mediaTimer);
    unlistenMediaControl();
  });
}
