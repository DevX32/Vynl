import { getCurrentTrack, getCurrentTime, getIsTrackPlaying } from "@state/now-playing.svelte";
import { convertFileSrc } from "@tauri-apps/api/core";
import { vynl } from "@lib/vynl";
import { evictOldest } from "@lib/cache";
import { clamp01 } from "@lib/pointer";
import type { MediaControlEvent, MediaUpdate } from "@lib/types";

type CurrentMedia = ReturnType<typeof getCurrentTrack>;

let lastMediaSentAt = 0;
const mediaCoverPaths = new Map<string, string>();
let pendingMedia: MediaUpdate | null = null;
let flushingMedia = false;
let mediaFlush: Promise<void> = Promise.resolve();
let lastMediaUpdateWarningAt = 0;
let lastMediaAction: { action: string; at: number } | null = null;

type Deps = {
  toggle: () => void;
  next: () => void;
  prev: () => void;
  pausePlayback: () => void;
  seekToTime: (time: number) => void;
  seekOffset: (delta: number) => void;
  setVolumeAt: (ratio: number) => void;
  getCurrentVolume: () => number;
  toggleMute: () => void;
};
let deps: Deps = {
  toggle: () => {}, next: () => {}, prev: () => {}, pausePlayback: () => {},
  seekToTime: () => {}, seekOffset: () => {}, setVolumeAt: () => {},
  getCurrentVolume: () => 1, toggleMute: () => {},
};
export function initMedia(d: Deps): void { deps = d; }

export function clearMediaCoverCache(): void { mediaCoverPaths.clear(); }

async function resolveMediaCover(cover: string | null): Promise<string | null> {
  if (!cover) return null;
  if (!/^https?:\/\//i.test(cover)) return cover;
  const cached = mediaCoverPaths.get(cover);
  if (cached) return cached;
  try {
    const local = await vynl.cacheCover(cover);
    mediaCoverPaths.set(cover, local);
    evictOldest(mediaCoverPaths, 200);
    return local;
  } catch {
    return cover;
  }
}

function warnMediaUpdateFailed(error: unknown): void {
  const now = Date.now();
  if (now - lastMediaUpdateWarningAt < 60_000) return;
  lastMediaUpdateWarningAt = now;
  console.warn("[vynl] media update failed:", error);
}

function flushMediaUpdates(): Promise<void> {
  if (flushingMedia) return mediaFlush;
  flushingMedia = true;
  mediaFlush = (async () => {
    while (pendingMedia) {
      const payload = pendingMedia;
      pendingMedia = null;
      try {
        if (payload.cover) payload.cover = await resolveMediaCover(payload.cover);
        await vynl.mediaUpdate(payload);
      } catch (error) {
        warnMediaUpdateFailed(error);
      }
    }
  })().finally(() => {
    flushingMedia = false;
  });
  return mediaFlush;
}

function sendMediaUpdate(np: CurrentMedia, playing: boolean, position: number): void {
  pendingMedia = np
    ? { title: np.title, artist: np.artist, album: np.album, duration: np.duration, cover: np.cover, playing, position }
    : null;
  void flushMediaUpdates();
}

export function syncMedia(np: CurrentMedia, playing: boolean, position: number): void {
  lastMediaSentAt = Date.now();
  sendMediaUpdate(np, playing, position);
}

export function syncMediaNow(): void {
  syncMedia(getCurrentTrack(), getIsTrackPlaying(), getCurrentTime());
}

export function getLastMediaSentAt(): number { return lastMediaSentAt; }

export function handleMediaPlay(): void { if (!getIsTrackPlaying()) deps.toggle(); }
export function handleMediaPause(): void { if (getIsTrackPlaying()) deps.toggle(); }
export function handleMediaStop(): void { if (getIsTrackPlaying()) deps.pausePlayback(); deps.seekToTime(0); }

const IDEMPOTENT_MEDIA_ACTIONS = new Set([
  "play", "pause", "toggle", "mute", "stop",
]);

export function handleMediaControl(event: MediaControlEvent): void {
  if (IDEMPOTENT_MEDIA_ACTIONS.has(event.action)) {
    const now = Date.now();
    if (lastMediaAction?.action === event.action && now - lastMediaAction.at < 150) return;
    lastMediaAction = { action: event.action, at: now };
  }
  switch (event.action) {
    case "play": handleMediaPlay(); break;
    case "pause": handleMediaPause(); break;
    case "toggle": deps.toggle(); break;
    case "next": deps.next(); break;
    case "previous": deps.prev(); break;
    case "stop": handleMediaStop(); break;
    case "seek": deps.seekToTime(event.position); break;
    case "seekBy": deps.seekOffset(event.offset); break;
    case "setVolume": deps.setVolumeAt(clamp01(event.volume)); break;
    case "volumeUp": deps.setVolumeAt(deps.getCurrentVolume() + 0.05); break;
    case "volumeDown": deps.setVolumeAt(deps.getCurrentVolume() - 0.05); break;
    case "mute": deps.toggleMute(); break;
  }
}


const IS_MINI = typeof window !== "undefined" && window.location.hash.startsWith("#mini");

export function updateMediaSession(): void {
  if (!("mediaSession" in navigator) || IS_MINI) return;
  const np = getCurrentTrack();
  if (!np) {
    navigator.mediaSession.metadata = null;
    navigator.mediaSession.playbackState = "none";
    return;
  }
  const meta: MediaMetadataInit = {
    title: np.title,
    artist: np.artist,
    album: np.album || undefined,
  };
  if (np.cover) {
    meta.artwork = [
      {
        src: /^https?:\/\//i.test(np.cover)
          ? np.cover
          : convertFileSrc(np.cover),
      },
    ];
  }
  navigator.mediaSession.metadata = new MediaMetadata(meta);
  navigator.mediaSession.playbackState = getIsTrackPlaying()
    ? "playing"
    : "paused";
}
