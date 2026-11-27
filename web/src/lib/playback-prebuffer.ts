import { prefetchLyrics } from "@state/lyrics.svelte";
import {
  getCurrentTrack,
  findTrackById,
  peekNextId,
  peekPrevId,
  getUserQueuePaths,
  getContextQueuePaths,
  getContextIndex,
} from "@state/now-playing.svelte";
import { getCurrentLoop, getCurrentShuffle } from "@state/player.svelte";
import { vynl } from "@lib/vynl";
import { catalogueIdFromPath as catalogueIdOf } from "@lib/catalogue";

interface PrebufferEntry {
  status: "pending" | "ready" | "failed";
  at: number;
}

interface PrebufferTarget {
  videoId: string;
  duration?: number;
}

export const prebufferState = new Map<string, PrebufferEntry>();
const PREBUFFER_REFRESH_MS = 45_000;
let prebufferCandidates: string[] = [];

function requestPrebuffer(target: PrebufferTarget, force: boolean): void {
  const now = Date.now();
  const entry = prebufferState.get(target.videoId);
  if (!force && entry && now - entry.at < PREBUFFER_REFRESH_MS) return;
  if (entry?.status === "pending") return;

  prebufferState.set(target.videoId, { status: "pending", at: now });
  void vynl
    .cataloguePrebuffer(target.videoId, target.duration)
    .then(() => {
      if (!prebufferCandidates.includes(target.videoId)) return;
      prebufferState.set(target.videoId, { status: "ready", at: Date.now() });
    })
    .catch((error) => {
      if (!prebufferCandidates.includes(target.videoId)) return;
      prebufferState.set(target.videoId, { status: "failed", at: Date.now() });
      console.warn("[vynl] prebuffer failed:", error);
    });
}

export function schedulePrebuffers(force = false): void {
  if (typeof window === "undefined") return;
  if (getCurrentLoop() === "one") return;
  const currentId = getCurrentTrack()?.id ?? null;

  const nextTrack = findTrackById(
    peekNextId(
      getUserQueuePaths(),
      getContextQueuePaths(),
      getContextIndex(),
      currentId,
      getCurrentShuffle(),
      getCurrentLoop(),
    ),
  );
  const prevTrack = findTrackById(
    peekPrevId(
      currentId,
      getContextQueuePaths(),
      getContextIndex(),
      getCurrentShuffle(),
      getCurrentLoop(),
    ),
  );

  if (nextTrack) prefetchLyrics(nextTrack);

  const targets: PrebufferTarget[] = [];
  for (const track of [nextTrack, prevTrack]) {
    const videoId = catalogueIdOf(track?.path);
    if (!videoId) continue;
    if (targets.some((target) => target.videoId === videoId)) continue;
    targets.push({ videoId, duration: track?.duration });
  }

  for (const stale of prebufferCandidates) {
    if (targets.some((target) => target.videoId === stale)) continue;
    prebufferState.delete(stale);
    void vynl.catalogueDiscardPrebuffer(stale).catch(() => {});
  }
  prebufferCandidates = targets.map((target) => target.videoId);

  for (const target of targets) requestPrebuffer(target, force);
}
