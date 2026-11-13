import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import {
  toggle,
  next,
  prev,
  setVolumeAt,
  seekToTime,
} from "@lib/player.svelte";
import { toggleMute } from "@state/player.svelte";
import {
  MINI_COMMAND_EVENT,
  MINI_REQUEST_STATE_EVENT,
  sendMiniState,
} from "@lib/mini-sync";
import { getCurrentTrack, getIsTrackPlaying } from "@state/now-playing.svelte";
import type { MiniCommand, MiniSyncState } from "@lib/types";

function currentSnapshot(): MiniSyncState {
  const track = getCurrentTrack();
  return {
    track: track
      ? {
          id: track.id,
          path: track.path,
          title: track.title,
          artist: track.artist,
          album: track.album,
          cover: track.cover,
          duration: track.duration,
        }
      : null,
    playing: getIsTrackPlaying(),
  };
}

export function initMiniBridge(): () => void {
  let offCommand: (() => void) | null = null;
  let offRequest: (() => void) | null = null;
  let disposed = false;

  const applyCommand = (cmd: MiniCommand): void => {
    if (!cmd || typeof cmd !== "object") return;
    switch (cmd.type) {
      case "toggle":
        toggle();
        break;
      case "next":
        next();
        break;
      case "prev":
        prev();
        break;
      case "toggleMute":
        toggleMute();
        break;
      case "seek":
        if (Number.isFinite(cmd.value)) seekToTime(cmd.value);
        break;
      case "volume":
        if (Number.isFinite(cmd.value)) setVolumeAt(cmd.value);
        break;
      case "expand":
        void (async () => {
          try {
            const win = getCurrentWindow();
            await win.unminimize();
            await win.show();
            await win.setFocus();
          } catch {

          }
        })();
        break;
    }
  };

  void listen<MiniCommand>(MINI_COMMAND_EVENT, (e) => applyCommand(e.payload)).then(
    (fn) => {
      if (disposed) fn();
      else offCommand = fn;
    },
  );

  void listen<unknown>(MINI_REQUEST_STATE_EVENT, () => {
    sendMiniState(currentSnapshot());
  }).then((fn) => {
    if (disposed) fn();
    else offRequest = fn;
  });

  sendMiniState(currentSnapshot());

  return () => {
    disposed = true;
    offCommand?.();
    offRequest?.();
  };
}
