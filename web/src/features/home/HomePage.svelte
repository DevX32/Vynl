<script lang="ts">
  import { Play, Shuffle, Link2, Music2, Library } from "lucide-svelte";
  import { getLibrary } from "@state/library.svelte";
  import { getPlaylistStreamTracks } from "@state/playlists.svelte";
  import type { LibraryTrack } from "@lib/types";
  import {
    requestPlay,
    getCurrentId,
    matchesCurrentSong,
  } from "@state/now-playing.svelte";
  import { setShuffle } from "@state/player.svelte";
  import { blobSrc, isSameSong } from "../../lib/format";
  import { QUOTES } from "@lib/constants";
  import { resolveDisplayName } from "@lib/nickname.svelte";
  import { t } from "@lib/i18n";
  import { getCurrentSettings } from "@state/settings.svelte";

  let {
    onNavigate,
  }: {
    onNavigate: (page: "player" | "vault" | "library") => void;
  } = $props();

  const library = $derived(getLibrary());
  const playlistStreams = $derived(getPlaylistStreamTracks());
  const currentId = $derived(getCurrentId());

  function isCurrent(track: {
    id: string;
    title: string;
    artist: string;
    duration: number;
  }): boolean {
    if (track.id === currentId) return true;
    return matchesCurrentSong(track.title, track.artist, track.duration);
  }

  let _tick = $state(0);

  const greeting = $derived.by(() => {
    void _tick;
    const h = new Date().getHours();
    let base: string;
    if (h < 6) base = t("home.greeting.night");
    else if (h < 12) base = t("home.greeting.morning");
    else if (h < 18) base = t("home.greeting.afternoon");
    else base = t("home.greeting.evening");

    const name = resolveDisplayName(getCurrentSettings().displayName);
    return { base, name };
  });

  const quote = $derived.by(() => {
    void _tick;
    return QUOTES[(new Date().getDate() - 1) % QUOTES.length];
  });

  $effect(() => {
    void _tick;
    const now = new Date();
    const msUntilMidnight =
      new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1).getTime() -
      now.getTime();
    const timer = setTimeout(() => {
      _tick++;
    }, msUntilMidnight);
    return () => clearTimeout(timer);
  });

  const allTracks = $derived.by(() => {
    const seen: LibraryTrack[] = [];
    for (const track of [...library, ...playlistStreams]) {
      if (seen.some((known) => known.id === track.id || isSameSong(known, track)))
        continue;
      seen.push(track);
    }
    return seen;
  });

  const recentTracks = $derived(
    [...allTracks]
      .sort((a, b) => {
        const aAdded = a.addedAt ?? a.mtime ?? 0;
        const bAdded = b.addedAt ?? b.mtime ?? 0;
        return (
          bAdded - aAdded ||
          (b.mtime ?? 0) - (a.mtime ?? 0) ||
          b.id.localeCompare(a.id)
        );
      })
      .slice(0, 7),
  );

  const featuredTrack = $derived.by(() => {
    if (allTracks.length === 0) return null;
    const now = new Date();
    return allTracks[(now.getDate() * 31 + now.getMonth()) % allTracks.length];
  });

  function shuffleAll(): void {
    if (allTracks.length === 0) return;
    setShuffle(true);
    const first = allTracks[Math.floor(Math.random() * allTracks.length)];
    requestPlay(
      first.id,
      allTracks.map((track) => track.path),
    );
  }

  function playTrack(id: string): void {
    requestPlay(
      id,
      allTracks.map((track) => track.path),
    );
  }
</script>

<section class="home-page">
  <div class="home-scroll">
    <header class="home-hdr">
      <span class="greeting display">{greeting.base}{#if greeting.name},
      <span class="greeting-name">{greeting.name}</span>{/if}</span>
      <span class="quote">{quote}</span>
    </header>

    {#if allTracks.length > 0}
      {#if featuredTrack}
        <button
          class="featured"
          onclick={() => playTrack(featuredTrack!.id)}
          aria-label={t("home.jumpBackIn")}
        >
          <div class="featured-art">
            {#if featuredTrack.cover}
              <img
                class="featured-art-img"
                use:blobSrc={featuredTrack.cover}
                alt=""
                draggable="false"
              />
            {:else}
              <div class="featured-art-placeholder">
                <Music2 size={40} stroke-width={1} />
              </div>
            {/if}
          </div>
          <div class="featured-scrim"></div>
          <div class="featured-body">
            <span class="featured-label">{t("home.jumpBackIn")}</span>
            <span class="featured-title">{featuredTrack.title}</span>
            <span class="featured-artist">{featuredTrack.artist}</span>
          </div>
          <div class="featured-play">
            <Play size={16} fill="currentColor" stroke-width={0} />
          </div>
        </button>
      {/if}

      <div class="actions">
        <button class="action-btn" onclick={shuffleAll}>
          <Shuffle size={14} stroke-width={1.5} />
          <span>{t("home.shuffleAll")}</span>
        </button>
        <button class="action-btn" onclick={() => onNavigate("vault")}>
          <Link2 size={14} stroke-width={1.5} />
          <span>{t("home.addMusic")}</span>
        </button>
          <button class="action-btn" onclick={() => onNavigate("library")}>
            <Library size={14} stroke-width={1.5} />
            <span>{t("nav.library")}</span>
          </button>
      </div>

      {#if recentTracks.length > 0}
        <div class="section">
          <div class="section-head">
            <span class="section-label">{t("home.recentlyAdded")}</span>
            <button class="see-all" onclick={() => onNavigate("library")}
              >{t("home.viewAll")}</button
            >
          </div>
          <div class="recent-grid">
            {#each recentTracks as track (track.id)}
              <button class="recent-card" onclick={() => playTrack(track.id)}>
                <div class="recent-art">
                  {#if track.cover}
                    <img
                      class="recent-art-img"
                      use:blobSrc={track.cover}
                      alt=""
                      loading="lazy"
                      draggable="false"
                    />
                  {:else}
                    <div class="recent-art-placeholder">
                      <Music2 size={18} stroke-width={1.2} />
                    </div>
                  {/if}
                </div>
                <span class="recent-title" class:playing={isCurrent(track)}
                  >{track.title}</span
                >
                <span class="recent-artist">{track.artist}</span>
              </button>
            {/each}
          </div>
        </div>
      {/if}
    {:else}
        <div class="empty-state">
        <Music2 size={32} stroke-width={1} />
        <span class="empty-title display">{t("home.emptyTitle")}</span>
        <span class="empty-sub">{t("home.emptySub")}</span>
        <button class="action-btn" onclick={() => onNavigate("vault")}>
          <Link2 size={14} stroke-width={1.5} />
          <span>{t("home.openVault")}</span>
        </button>
      </div>
    {/if}
  </div>
</section>

<style>
  .home-page {
    height: 100%;
    overflow: hidden;
    overflow-x: hidden;
    contain: layout;
  }

  .home-scroll {
    height: 100%;
    overflow-y: auto;
    scrollbar-width: none;
    padding: 28px 0 40px;
  }

  .home-scroll::-webkit-scrollbar {
    display: none;
  }

  .home-hdr {
    display: flex;
    flex-direction: column;
    margin-bottom: 25px;
  }

  .greeting {
    font-size: 40px;
    line-height: 1.2;
    color: var(--text);
    margin-bottom: 6px;
  }

  .greeting-name {
    font-family: var(--font-serif);
    font-style: italic;
    color: var(--accent);
  }

  .quote {
    font-size: 13px;
    font-style: italic;
    color: var(--dim);
    border-left: 2px solid var(--line-strong);
    padding-left: 10px;
  }

  .featured {
    position: relative;
    display: block;
    width: 100%;
    height: 182px;
    border-radius: var(--radius-sm);
    text-align: left;
    margin-bottom: 20px;
    background: var(--bg);
  }

  .featured-art {
    position: absolute;
    inset: 0;
    overflow: hidden;
    clip-path: inset(0 round var(--radius-sm));
    border-radius: var(--radius-sm);
    background: var(--bg);
  }

  .featured-art-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transform: scale(1.02);
    filter: saturate(1.25) brightness(0.9) blur(2px);
  }

  .featured-art-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--faint);
    background: var(--placeholder-gradient);
  }

  .featured-scrim {
    position: absolute;
    inset: 0;
    border-radius: var(--radius-sm);
    pointer-events: none;
    background:
      linear-gradient(
        180deg,
        transparent 0%,
        transparent 28%,
        rgba(8, 8, 12, 0.28) 55%,
        rgba(8, 8, 12, 0.72) 82%,
        rgba(8, 8, 12, 0.9) 100%
      ),
      radial-gradient(
        ellipse 70% 90% at 0% 100%,
        rgba(0, 0, 0, 0.45) 0%,
        transparent 65%
      );
  }

  .featured-body {
    position: absolute;
    left: 20px;
    right: 76px;
    bottom: 18px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .featured-label {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.75);
    margin-bottom: 4px;
  }

  .featured-title {
    font-size: 18px;
    font-weight: 600;
    color: #fff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .featured-artist {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.7);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .featured-play {
    position: absolute;
    right: 22px;
    bottom: 22px;
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--accent-text);
    display: flex;
    align-items: center;
    justify-content: center;
    transform: rotate(45deg);
    transition:
      background 0.15s,
      transform 0.1s,
      box-shadow 0.15s;
  }

  .featured-play :global(svg) {
    transform: rotate(-45deg);
  }

  .featured:hover .featured-play {
    background: color-mix(in srgb, var(--accent) 82%, #ffffff);
    transform: rotate(45deg);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .featured:active .featured-play {
    transform: rotate(45deg) scale(0.92);
  }

  .section {
    margin-bottom: 28px;
  }

  .section-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  .section-label {
    font-size: 13px;
    color: var(--dim);
  }

  .see-all {
    font-size: 11px;
    color: var(--faint);
    transition: color 0.15s;
  }

  .see-all:hover {
    color: var(--text);
  }

  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 28px;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 7px 14px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line);
    font-size: 11px;
    color: var(--dim);
    transition:
      background 0.15s,
      color 0.15s,
      border-color 0.15s;
  }

  .action-btn:hover {
    background: var(--surface);
    color: var(--text);
    border-color: var(--line-strong);
  }

  .recent-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }

  .recent-card {
    display: flex;
    flex-direction: column;
    padding: 4px;
    border-radius: var(--radius-sm);
    text-align: left;
    flex: 1 1 120px;
    max-width: 160px;
    min-width: 100px;
    transition: background 0.15s, flex-basis 0.2s ease, max-width 0.2s ease;
  }

  .recent-card:hover {
    background: var(--bg-raise);
  }

  .recent-art {
    width: 100%;
    aspect-ratio: 1;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg-raise);
  }

  .recent-art-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .recent-art-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--faint);
    background: var(--placeholder-gradient);
  }

  .recent-title {
    margin-top: 8px;
    font-size: 12.5px;
    letter-spacing: -0.1px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text);
    transition: color 0.15s;
  }

  .recent-title.playing {
    color: var(--accent);
  }

  .recent-artist {
    margin-top: 2px;
    font-size: 11px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--faint);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 60px 20px;
    text-align: center;
    color: var(--faint);
  }

  .empty-title {
    font-size: 18px;
    color: var(--dim);
  }

  .empty-sub {
    font-size: 12px;
    color: var(--faint);
    max-width: 280px;
  }

  @container player (max-width: 620px) {
    .home-scroll {
      padding: 20px 0 32px;
    }

    .home-hdr {
      margin-bottom: 20px;
    }

    .greeting {
      font-size: 24px;
    }

    .featured {
      height: 150px;
      margin-bottom: 16px;
    }
  }

  @container player (max-width: 420px) {
    .greeting {
      font-size: 20px;
    }

    .empty-state {
      padding: 32px 16px;
    }
  }
</style>