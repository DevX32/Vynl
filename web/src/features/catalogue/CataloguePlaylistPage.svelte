<script lang="ts">
  import { Play, Shuffle, ListMusic, AlertCircle, LoaderCircle } from "lucide-svelte";
  import type { CataloguePlaylistDetail, CatalogueTrack, Page } from "@lib/types";
  import {
    appendCataloguePlaylistPage,
    cataloguePath,
    loadCataloguePlaylist,
  } from "@state/catalogue.svelte";
  import { requestPlay } from "@state/now-playing.svelte";
  import { setShuffle } from "@state/player.svelte";
  import { blobSrc, fmtTotalDuration, totalSeconds } from "@lib/format";
  import { t } from "@lib/i18n";
  import { friendlyError } from "@lib/errors";
  import { vynl } from "@lib/vynl";
  import CatalogueTrackList from "./CatalogueTrackList.svelte";

  let {
    id,
    onNavigate,
  }: {
    id: string;
    onNavigate: (page: Page) => void;
  } = $props();

  let detail = $state<CataloguePlaylistDetail | null>(null);
  let loading = $state(false);
  let loadingMore = $state(false);
  let error = $state<string | null>(null);
  let moreError = $state<string | null>(null);
  let loadToken = 0;

  const playlist = $derived(detail?.playlist ?? null);
  const tracks = $derived(detail?.tracks ?? []);
  const totalDur = $derived(totalSeconds(tracks));

  async function load(playlistId: string, force = false): Promise<void> {
    if (!playlistId) return;
    const token = ++loadToken;
    loading = true;
    error = null;
    moreError = null;
    try {
      const result = await loadCataloguePlaylist(playlistId, force);
      if (token === loadToken) detail = result;
    } catch (err) {
      if (token === loadToken) {
        error = err instanceof Error ? err.message : String(err);
      }
    } finally {
      if (token === loadToken) loading = false;
    }
  }

  $effect(() => {
    void load(id);
  });

  function play(shuffle = false): void {
    if (tracks.length === 0) return;
    if (shuffle) setShuffle(true);
    const first = shuffle
      ? tracks[Math.floor(Math.random() * tracks.length)]
      : tracks[0];
    requestPlay(
      first.id,
      tracks.map((track) => cataloguePath(track.id)),
    );
  }

  function playTrack(track: CatalogueTrack): void {
    requestPlay(
      track.id,
      tracks.map((item) => cataloguePath(item.id)),
    );
  }

  async function loadMore(): Promise<void> {
    const token = detail?.continuation;
    if (!token || loadingMore) return;
    loadingMore = true;
    moreError = null;
    try {
      const page = await vynl.cataloguePlaylistPage(token);
      const merged = appendCataloguePlaylistPage(id, page);
      if (merged) detail = merged;
    } catch (err) {
      moreError = err instanceof Error ? err.message : String(err);
    } finally {
      loadingMore = false;
    }
  }
</script>

<div class="detail-page">
  {#if detail && playlist}
    <div class="hero">
      <div class="hero-cover">
        {#if playlist.cover}
          <img use:blobSrc={playlist.cover} alt="" draggable="false" />
        {:else}
          <div class="hero-cover-placeholder">
            <ListMusic size={30} stroke-width={1} />
          </div>
        {/if}
      </div>

      <div class="hero-meta">
        <div class="kicker mono">{t("catalogue.playlists")}</div>
        <div class="hero-title display">{playlist.title}</div>
        {#if playlist.author}
          <div class="hero-sub mono">{playlist.author}</div>
        {/if}
        <div class="hero-sub mono">
          {playlist.trackCount ?? tracks.length}
          {(playlist.trackCount ?? tracks.length) === 1
            ? t("catalogue.track")
            : t("catalogue.tracks")}
          {#if totalDur > 0} · {fmtTotalDuration(totalDur)}{/if}
        </div>
      </div>

      <div class="hero-actions">
        <button
          class="play-big"
          onclick={() => play(false)}
          disabled={tracks.length === 0}
          aria-label={t("catalogue.playAll")}
        >
          <Play size={16} fill="currentColor" stroke-width={0} />
        </button>
        <button
          class="icon-btn round"
          onclick={() => play(true)}
          disabled={tracks.length === 0}
          aria-label={t("catalogue.shuffle")}
        >
          <Shuffle size={14} stroke-width={1.5} />
        </button>
      </div>
    </div>
  {/if}

  <div class="detail-scroll">
    {#if loading && !detail}
      <div class="state">
        <span class="spin"><LoaderCircle size={22} stroke-width={1.5} /></span>
        <span class="state-sub mono">{t("common.loading")}</span>
      </div>
    {:else if error && !detail}
      <div class="state" role="alert">
        <AlertCircle size={26} stroke-width={1.2} />
        <span class="state-title display">{t("catalogue.loadFailed")}</span>
        <span class="state-sub mono">{friendlyError(error)}</span>
        <button class="retry-btn" onclick={() => void load(id, true)}>
          {t("common.retry")}
        </button>
      </div>
    {:else if detail && playlist}
      <CatalogueTrackList
        {tracks}
        showAlbum={true}
        onPlayTrack={playTrack}
        onNavigateAlbum={(albumId) => onNavigate(`catalogue:album:${albumId}`)}
        onNavigateArtist={(artistId) => onNavigate(`catalogue:artist:${artistId}`)}
      />

      {#if detail.continuation}
        <div class="more">
          <button
            class="more-btn"
            disabled={loadingMore}
            onclick={() => void loadMore()}
          >
            {#if loadingMore}
              <span class="spin"><LoaderCircle size={13} stroke-width={1.5} /></span>
              {t("catalogue.loadingMore")}
            {:else}
              {t("catalogue.loadMore")}
            {/if}
          </button>
          {#if moreError}
            <span class="more-error mono">{moreError}</span>
          {/if}
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .detail-page {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .detail-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding-bottom: 36px;
  }

  .detail-scroll::-webkit-scrollbar {
    width: 6px;
  }

  .detail-scroll::-webkit-scrollbar-thumb {
    background: rgba(240, 240, 236, 0.12);
    border-radius: 3px;
  }

  .detail-scroll::-webkit-scrollbar-thumb:hover {
    background: rgba(240, 240, 236, 0.22);
  }

  .detail-scroll::-webkit-scrollbar-track {
    background: transparent;
  }

  .hero {
    display: flex;
    align-items: flex-end;
    gap: 20px;
    padding: 24px 0 16px;
    padding-right: 10px;
    flex-wrap: wrap;
    flex-shrink: 0;
  }

  .hero-cover {
    width: 150px;
    height: 150px;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg-raise);
    box-shadow:
      0 0 0 1px rgba(255, 255, 255, 0.07),
      var(--shadow-md);
  }

  .hero-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .hero-cover-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--faint);
    background: var(--placeholder-gradient);
  }

  .hero-meta {
    flex: 1;
    min-width: 0;
  }

  .kicker {
    font-size: 10px;
    letter-spacing: 0.16em;
    color: var(--faint);
    text-transform: uppercase;
  }

  .hero-title {
    font-size: 34px;
    line-height: 1.05;
    letter-spacing: -0.02em;
    margin-top: 5px;
    overflow-wrap: anywhere;
  }

  .hero-sub {
    margin-top: 7px;
    font-size: 11px;
    color: var(--faint);
    letter-spacing: 0.04em;
    overflow-wrap: anywhere;
  }

  .hero-actions {
    display: flex;
    align-items: center;
    gap: 18px;
    flex-shrink: 0;
    margin-bottom: 2px;
  }

  .play-big {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent);
    color: var(--accent-text);
    border: none;
    cursor: pointer;
    transform: rotate(45deg);
    transition: transform 0.15s, background 0.15s;
  }

  .play-big :global(svg) {
    transform: rotate(-45deg);
  }

  .play-big:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 82%, #ffffff);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .play-big:active:not(:disabled) {
    transform: rotate(45deg) scale(0.92);
  }

  .play-big:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .icon-btn.round {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line-strong);
    background: var(--bg-raise);
    color: var(--dim);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transform: rotate(45deg);
    transition: color 0.15s, border-color 0.15s, background 0.15s;
  }

  .icon-btn.round :global(svg) {
    transform: rotate(-45deg);
  }

  .icon-btn.round:hover:not(:disabled) {
    color: var(--text);
    background: var(--surface);
  }

  .icon-btn.round:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .more {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 18px 0 8px;
  }

  .more-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 8px 20px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line-strong);
    background: var(--bg-raise);
    color: var(--dim);
    font-size: 11.5px;
    cursor: pointer;
  }

  .more-btn:hover:not(:disabled) {
    color: var(--text);
    background: var(--surface);
  }

  .more-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .more-error {
    font-size: 10.5px;
    color: var(--red);
    max-width: 520px;
    text-align: center;
    overflow-wrap: anywhere;
  }

  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    min-height: 320px;
    padding: 40px 20px;
    text-align: center;
    color: var(--faint);
  }

  .state-title {
    font-size: 23px;
    color: var(--dim);
  }

  .state-sub {
    font-size: 11px;
    letter-spacing: 0.04em;
    max-width: 420px;
    overflow-wrap: anywhere;
  }

  .retry-btn {
    margin-top: 4px;
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line-strong);
    background: var(--bg-raise);
    color: var(--dim);
    font-size: 11px;
    cursor: pointer;
  }

  .retry-btn:hover {
    color: var(--text);
  }

  .spin {
    display: inline-flex;
    align-items: center;
    animation: spin 0.85s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @container player (max-width: 620px) {
    .hero {
      align-items: center;
      gap: 14px;
      padding-top: 16px;
    }

    .hero-cover {
      width: 112px;
      height: 112px;
    }

    .hero-title {
      font-size: 27px;
    }
  }

  @container player (max-width: 420px) {
    .hero-cover {
      width: 92px;
      height: 92px;
    }

    .hero-title {
      font-size: 22px;
    }

    .hero-actions {
      gap: 12px;
    }
  }
</style>
