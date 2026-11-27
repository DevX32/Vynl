import type { StreamErrorEvent } from "@lib/types";

export const RECONCILE_STALE_MS = 10_000;
export const CROSSFADE_TAIL = 0.75;

export const playbackPhase = {
  pendingPlayGeneration: null as number | null,
  playReconcileStartedAt: 0,
  crossfadeTried: false,
  pendingStreamError: null as StreamErrorEvent | null,
};

export function beginPlayReconcile(generation: number): void {
  playbackPhase.pendingPlayGeneration = generation;
  playbackPhase.playReconcileStartedAt = Date.now();
}

export function finishPlayReconcile(generation: number): void {
  if (playbackPhase.pendingPlayGeneration === generation) {
    playbackPhase.pendingPlayGeneration = null;
    playbackPhase.playReconcileStartedAt = 0;
  }
}

export function reconcileIsStale(): boolean {
  return (
    playbackPhase.pendingPlayGeneration !== null &&
    Date.now() - playbackPhase.playReconcileStartedAt >= RECONCILE_STALE_MS
  );
}

export function resetCrossfade(): void {
  playbackPhase.crossfadeTried = false;
}
