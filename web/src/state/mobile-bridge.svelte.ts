import { vynl } from "@lib/vynl";
import { getCurrentLoop, getCurrentShuffle } from "@state/player.svelte";
import {
  getContextUpcoming,
  getCurrentTrack,
} from "@state/now-playing.svelte";
import type {
  LibraryTrack,
  MobileNowPlaying,
  MobileTrack,
  NowPlaying,
} from "@lib/types";

function toMobileTrack(track: LibraryTrack | NowPlaying): MobileTrack {
  return {
    id: track.id,
    title: track.title,
    artist: track.artist,
    album: track.album || null,
    duration: track.duration > 0 ? track.duration : null,
    cover: track.cover ?? null,
  };
}

let publishedKey = "";
let publishing = false;

function publish(): void {
  if (publishing) return;
  const current = getCurrentTrack();
  const upcoming = getContextUpcoming(getCurrentShuffle(), getCurrentLoop());
  const key = `${current?.id ?? ""}|${upcoming.map((t) => t.id).join(",")}`;
  if (key === publishedKey) return;
  publishedKey = key;
  publishing = true;

  const state: MobileNowPlaying = {
    track: current ? toMobileTrack(current) : null,
    queue: upcoming.slice(0, 50).map(toMobileTrack),
  };

  void vynl
    .mobileSyncSetNowPlaying(state)
    .catch(() => {})
    .finally(() => {
      publishing = false;
    });
}

export function startMobileBridge(): () => void {
  publish();
  const timer = window.setInterval(publish, 2_000);
  return () => {
    window.clearInterval(timer);
  };
}
