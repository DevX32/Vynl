<script lang="ts">
  import { Play, Shuffle, User, AlertCircle, Loader } from "lucide-svelte";
  import type { CatalogueArtist, CatalogueTrack, Page } from "@lib/types";
  import { cataloguePath, loadCatalogueArtist } from "@state/catalogue.svelte";
  import { requestPlay } from "@state/now-playing.svelte";
  import { setShuffle } from "@state/player.svelte";
  import { blobSrc } from "@lib/format";
  import { t } from "@lib/i18n";
  import { friendlyError } from "@lib/errors";
  import CatalogueTrackList from "./CatalogueTrackList.svelte";
  import CatalogueCard from "./CatalogueCard.svelte";

  let {
    id,
    onNavigate,
  }: {
    id: string;
    onNavigate: (page: Page) => void;
  } = $props();

  let artist = $state<CatalogueArtist | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let loadToken = 0;

  const topTracks = $derived(artist?.topTracks ?? []);

  async function load(artistId: string, force = false): Promise<void> {
    if (!artistId) return;
    const token = ++loadToken;
    loading = true;
    error = null;
    try {
      const result = await loadCatalogueArtist(artistId, force);
      if (token === loadToken) artist = result;
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
    if (topTracks.length === 0) return;
    if (shuffle) setShuffle(true);
    const first = shuffle
      ? topTracks[Math.floor(Math.random() * topTracks.length)]
      : topTracks[0];
    requestPlay(
      first.id,
      topTracks.map((track) => cataloguePath(track.id)),
    );
  }

  function playTrack(track: CatalogueTrack): void {
    requestPlay(
      track.id,
      topTracks.map((item) => cataloguePath(item.id)),
    );
  }
</script>

<div class="detail-page">
  {#if artist}
    <div class="hero">
      <div class="hero-cover">
        {#if artist.cover}
          <img use:blobSrc={artist.cover} alt="" draggable="false" />
        {:else}
          <div class="hero-cover-placeholder">
            <User size={30} stroke-width={1} />
          </div>
        {/if}
      </div>

      <div class="hero-meta">
        <div class="kicker mono">{t("catalogue.openArtist")}</div>
        <div class="hero-title display">{artist.name}</div>
        {#if artist.subscribers}
          <div class="hero-sub mono">{artist.subscribers}</div>
        {/if}
      </div>

      <div class="hero-actions">
        <button
          class="play-big"
          onclick={() => play(false)}
          disabled={topTracks.length === 0}
          aria-label={t("catalogue.playAll")}
          title={t("catalogue.playAll")}
        >
          <Play size={16} fill="currentColor" stroke-width={0} />
        </button>
        <button
          class="icon-btn round"
          onclick={() => play(true)}
          disabled={topTracks.length === 0}
          aria-label={t("catalogue.shuffle")}
          title={t("catalogue.shuffle")}
        >
          <Shuffle size={14} stroke-width={1.5} />
        </button>
      </div>
    </div>
  {/if}

  <div class="detail-scroll">
    {#if loading && !artist}
      <div class="state">
        <span class="spin"><Loader size={22} stroke-width={1.4} /></span>
        <span class="state-sub mono">{t("common.loading")}</span>
      </div>
    {:else if error && !artist}
      <div class="state" role="alert">
        <AlertCircle size={26} stroke-width={1.2} />
        <span class="state-title display">{t("catalogue.loadFailed")}</span>
        <span class="state-sub mono">{friendlyError(error)}</span>
        <button class="retry-btn" onclick={() => void load(id, true)}>
          {t("common.retry")}
        </button>
      </div>
    {:else if artist}
      {#if artist.description}
        <p class="description">{artist.description}</p>
      {/if}

      {#if topTracks.length > 0}
        <section class="section">
          <div class="section-head">
            <span class="section-label">{t("catalogue.topSongs")}</span>
          </div>
          <CatalogueTrackList
            tracks={topTracks}
            showAlbum={true}
            onPlayTrack={playTrack}
            onNavigateAlbum={(albumId) => onNavigate(`catalogue:album:${albumId}`)}
          />
        </section>
      {/if}

      {#if artist.albums.length > 0}
        <section class="section">
          <div class="section-head">
            <span class="section-label">{t("catalogue.albums")}</span>
          </div>
          <div class="card-grid">
            {#each artist.albums as album (album.id)}
              <CatalogueCard
                title={album.title}
                subtitle={album.year ? String(album.year) : ""}
                cover={album.cover}
                kind="album"
                openLabel={t("catalogue.openAlbum")}
                onopen={() => onNavigate(`catalogue:album:${album.id}`)}
              />
            {/each}
          </div>
        </section>
      {/if}

      {#if artist.singles.length > 0}
        <section class="section">
          <div class="section-head">
            <span class="section-label">{t("catalogue.singles")}</span>
          </div>
          <div class="card-grid">
            {#each artist.singles as album (album.id)}
              <CatalogueCard
                title={album.title}
                subtitle={album.year ? String(album.year) : ""}
                cover={album.cover}
                kind="album"
                openLabel={t("catalogue.openAlbum")}
                onopen={() => onNavigate(`catalogue:album:${album.id}`)}
              />
            {/each}
          </div>
        </section>
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
    font-size: 38px;
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

  .description {
    margin: 0 0 18px;
    max-width: 720px;
    font-size: 12px;
    line-height: 1.6;
    color: var(--dim);
    white-space: pre-line;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .section {
    margin-bottom: 26px;
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 8px;
  }

  .section-label {
    font-size: 12px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--dim);
  }

  .card-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(128px, 1fr));
    gap: 10px;
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
    animation: spin 1s linear infinite;
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
      font-size: 30px;
    }
  }

  @container player (max-width: 420px) {
    .hero-cover {
      width: 92px;
      height: 92px;
    }

    .hero-title {
      font-size: 24px;
    }

    .hero-actions {
      gap: 12px;
    }
  }
</style>
