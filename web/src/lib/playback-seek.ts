import { getCurrentTrack, getCurrentTime, getIsTrackPlaying, setDragTime, setSeekDragging, updateTime } from "@state/now-playing.svelte";
import { vynl } from "@lib/vynl";
import { catalogueIdFromPath as catalogueIdOf } from "@lib/catalogue";
import { normalizeDuration } from "./playback-utils";

export const seekState = {
  pending: false,
  requestId: 0,
  target: null as number | null,
};

let getPlayGeneration: () => number = () => 0;
let syncMediaNow: () => void = () => {};

export function initSeek(deps: { getPlayGeneration: () => number; syncMediaNow: () => void }): void {
  getPlayGeneration = deps.getPlayGeneration;
  syncMediaNow = deps.syncMediaNow;
}

function clampSeekTarget(time: number, total?: number): number | null {
  if (!Number.isFinite(time)) return null;
  const duration = total ?? getCurrentTrack()?.duration ?? 0;
  const lower = Math.max(0, time);
  return Number.isFinite(duration) && duration > 0
    ? Math.min(lower, duration)
    : lower;
}

function requestSeek(time: number): void {
  const target = clampSeekTarget(time);
  if (target === null) return;

  seekState.target = target;
  setDragTime(target);
  seekState.pending = true;
  const requestId = ++seekState.requestId;
  const generation = getPlayGeneration();
  const np = getCurrentTrack();
  const videoId = catalogueIdOf(np?.path);

  const seek = videoId
    ? vynl.catalogueSeekStream(
        videoId,
        target,
        np?.duration,
        generation,
        getIsTrackPlaying(),
        requestId,
      )
    : vynl.playerSeek(target, generation);

  void seek
    .then(() => {
      if (requestId !== seekState.requestId || generation !== getPlayGeneration()) return;
      seekState.pending = false;
      seekState.target = null;
      updateTime(target, true);
      syncMediaNow();
    })
    .catch((error) => {
      if (requestId !== seekState.requestId || generation !== getPlayGeneration()) return;
      seekState.pending = false;
      seekState.target = null;
      console.warn("[vynl] seek failed:", error);
      void vynl
        .playerGetPosition(generation)
        .then((position) => {
          if (
            Number.isFinite(position) &&
            requestId === seekState.requestId &&
            generation === getPlayGeneration()
          ) {
            updateTime(position);
          }
        })
        .catch(() => {});
    });
}

export function seekRatio(ratio: number, total?: number): void {
  if (!getCurrentTrack() || !Number.isFinite(ratio)) return;
  const duration = normalizeDuration(total);
  if (duration <= 0) {
    cancelSeek();
    return;
  }
  const target = clampSeekTarget(ratio * duration, duration);
  if (target === null) return;
  seekState.requestId += 1;
  seekState.pending = false;
  setDragTime(target);
  seekState.target = target;
}

export function seekFlush(): void {
  if (seekState.target !== null) requestSeek(seekState.target);
}

export function seekToTime(time: number): void {
  requestSeek(time);
}

export function seekOffset(delta: number): void {
  if (!Number.isFinite(delta)) return;
  const base = seekState.target ?? getCurrentTime();
  requestSeek(base + delta);
}

export function cancelSeek(restorePosition = true): void {
  seekState.requestId += 1;
  seekState.pending = false;
  seekState.target = null;
  setSeekDragging(false);
  if (restorePosition) {
    const generation = getPlayGeneration();
    void vynl
      .playerGetPosition(generation)
      .then((position) => {
        if (generation === getPlayGeneration() && Number.isFinite(position)) {
          updateTime(position);
        }
      })
      .catch(() => {});
  }
}

