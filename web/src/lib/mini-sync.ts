import { emit } from "@tauri-apps/api/event";
import type { LibraryTrack, MiniCommand, MiniSyncState } from "@lib/types";

type MiniTrackSource = Pick<
  LibraryTrack,
  "id" | "path" | "title" | "artist" | "album" | "cover" | "duration"
>;

export const MINI_STATE_EVENT = "vynl:mini:state";
export const MINI_COMMAND_EVENT = "vynl:mini:command";
export const MINI_REQUEST_STATE_EVENT = "vynl:mini:request-state";
export const MINI_CLOSED_EVENT = "vynl:mini:closed";

function safeEmit(event: string, payload: unknown): void {
  void emit(event, payload).catch((error) => {
    console.warn(`[vynl] mini ${event} emit failed`, error);
  });
}

export function miniTrackSnapshot(
  track: MiniTrackSource | null | undefined,
  playing: boolean,
): MiniSyncState {
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
    playing,
  };
}

export function sendMiniState(state: MiniSyncState): void {
  safeEmit(MINI_STATE_EVENT, state);
}

export function requestMiniState(): void {
  safeEmit(MINI_REQUEST_STATE_EVENT, null);
}

export function sendMiniCommand(command: MiniCommand): void {
  safeEmit(MINI_COMMAND_EVENT, command);
}

export function sendMiniClosed(): void {
  safeEmit(MINI_CLOSED_EVENT, null);
}
