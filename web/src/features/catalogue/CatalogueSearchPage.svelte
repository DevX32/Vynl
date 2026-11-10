<script lang="ts">
  import { untrack } from "svelte";
  import {
    Search,
    Play,
    Shuffle,
    AlertCircle,
    Loader,
  } from "lucide-svelte";
  import type { CatalogueTrack, Page } from "@lib/types";
  import {
    cataloguePath,
    getCatalogueQuery,
    getCatalogueResults,
    getCatalogueSearchError,
    getCatalogueTaste,
    getCatalogueTasteError,
    isCatalogueSearching,
    isCatalogueTasteLoading,
    loadCatalogueTaste,
    searchCatalogue,
  } from "@state/catalogue.svelte";
  import {
    getTasteProfileKey,
    getTasteSeeds,
    getTasteSummary,
  } from "@state/taste.svelte";
  import { requestPlay } from "@state/now-playing.svelte";
  import { setShuffle } from "@state/player.svelte";
  import { t } from "@lib/i18n";
  import { friendlyError } from "@lib/errors";
  import SearchInput from "@components/SearchInput.svelte";
  import CatalogueTrackList from "./CatalogueTrackList.svelte";
  import CatalogueSongCard from "./CatalogueSongCard.svelte";
  import CatalogueSongMenu from "./CatalogueSongMenu.svelte";

  let {
    onNavigate,
  }: {
    onNavigate: (page: Page) => void;
  } = $props();

  let query = $state(getCatalogueQuery());
  let requested = $state(getCatalogueQuery());
  let timer: ReturnType<typeof setTimeout> | undefined;
  let songMenu: CatalogueSongMenu;
  let mixGrid: HTMLDivElement | undefined = $state();
  let mixColumns = $state(1);

  const results = $derived(getCatalogueResults());
  const searching = $derived(isCatalogueSearching());
  const error = $derived(getCatalogueSearchError());

  const taste = $derived(getCatalogueTaste());
  const tasteLoading = $derived(isCatalogueTasteLoading());
  const tasteError = $derived(getCatalogueTasteError());
  const summary = $derived(getTasteSummary());
  const profileKey = $derived(getTasteProfileKey());

  $effect(() => {
    const key = profileKey;
    if (!key) return;
    const current = untrack(() => getTasteSeeds(6));
    if (current.length === 0) return;
    void loadCatalogueTaste(current);
  });

  const playableSongs = $derived(
    requested.trim() ? (results?.songs ?? []) : taste,
  );

  const mixTracks = $derived(
    mixColumns > 0
      ? taste.slice(0, Math.floor(taste.length / mixColumns) * mixColumns)
      : taste,
  );

  $effect(() => {
    const grid = mixGrid;
    if (!grid) return;
    const measure = (): void => {
      const template = getComputedStyle(grid).gridTemplateColumns;
      const columns = template && template !== "none" ? template.split(" ").length : 1;
      mixColumns = Math.max(1, columns);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(grid);
    return () => observer.disconnect();
  });

  $effect(() => {
    const value = query;
    clearTimeout(timer);
    timer = setTimeout(() => {
      const trimmed = value.trim();
      if (trimmed === requested) {
        if (results !== null || !trimmed || isCatalogueSearching()) return;
        void searchCatalogue(trimmed);
        return;
      }
      requested = trimmed;
      void searchCatalogue(trimmed);
    }, 320);
    return () => clearTimeout(timer);
  });

  function playList(tracks: readonly CatalogueTrack[], shuffle = false): void {
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

  function playTrack(track: CatalogueTrack, queue: readonly CatalogueTrack[]): void {
    requestPlay(
      track.id,
      queue.map((item) => cataloguePath(item.id)),
    );
  }

  function retry(): void {
    void searchCatalogue(query);
  }

  function retryTaste(): void {
    void loadCatalogueTaste(untrack(() => getTasteSeeds(6)), true);
  }
</script>

<div class="cat-page">
  <header class="page-hdr">
    <div class="hdr-copy">
      <div class="kicker mono">{t("catalogue.kicker")}</div>
      <div class="title display">{t("catalogue.searchTitle")}</div>
    </div>
    <div class="hdr-row">
      <SearchInput
        value={query}
        placeholder={t("catalogue.searchPlaceholder")}
        oninput={(value) => (query = value)}
      />
      {#if playableSongs.length > 0}
        <button
          class="play-btn"
          onclick={() => playList(playableSongs, false)}
          aria-label={t("catalogue.playAll")}
          title={t("catalogue.playAll")}
        >
          <Play size={16} fill="currentColor" stroke-width={0} />
        </button>
        <button
          class="shuffle-btn"
          onclick={() => playList(playableSongs, true)}
          aria-label={t("catalogue.shuffle")}
          title={t("catalogue.shuffle")}
        >
          <Shuffle size={14} stroke-width={1.5} />
        </button>
      {/if}
    </div>
  </header>

  <div class="page-scroll">
    {#if error}
      <div class="state" role="alert">
        <AlertCircle size={26} stroke-width={1.2} />
        <span class="state-title display">{t("catalogue.searchFailed")}</span>
        <span class="state-sub mono">{friendlyError(error)}</span>
        <button class="retry-btn" onclick={retry}>{t("common.retry")}</button>
      </div>
    {:else if searching && !results}
      <div class="state">
        <span class="spin"><Loader size={22} stroke-width={1.4} /></span>
        <span class="state-sub mono">{t("common.loading")}</span>
      </div>
    {:else if !requested.trim()}
      {#if summary.liked === 0 && summary.played === 0}
        <section class="fresh">
          <span class="state-title display">{t("catalogue.freshTitle")}</span>
          <span class="state-sub">{t("catalogue.freshHint")}</span>
        </section>
      {:else}
        {#if tasteError}
          <div class="mix-note" role="alert">
            <AlertCircle size={15} stroke-width={1.4} />
            <span>{t("catalogue.mixFailed")}</span>
            <button class="retry-btn" onclick={retryTaste}>
              {t("common.retry")}
            </button>
          </div>
        {:else if taste.length > 0}
          <section class="section">
            <div class="section-label mono">{t("catalogue.madeForYou")}</div>
            <div class="card-grid" bind:this={mixGrid}>
              {#each mixTracks as track (track.id)}
                <CatalogueSongCard
                  {track}
                  onplay={() => playTrack(track, taste)}
                  onmenu={(e, item) => songMenu.open(e, item)}
                />
              {/each}
            </div>
          </section>
        {:else if tasteLoading}
          <div class="mix-note">
            <span class="spin"><Loader size={14} stroke-width={1.4} /></span>
            <span>{t("catalogue.buildingMix")}</span>
          </div>
        {/if}
      {/if}
    {:else if results}
      {#if results.songs.length > 0}
        <section class="section">
          <CatalogueTrackList
            tracks={results.songs}
            onPlayTrack={(track) => playTrack(track, results.songs)}
            onNavigateAlbum={(id) => onNavigate(`catalogue:album:${id}`)}
            onNavigateArtist={(id) => onNavigate(`catalogue:artist:${id}`)}
          />
        </section>
      {/if}

      {#if results.songs.length === 0 &&
        results.albums.length === 0 &&
        results.playlists.length === 0}
        <div class="state">
          <Search size={32} stroke-width={1.1} />
          <span class="state-title display">{t("catalogue.noResults")}</span>
          <span class="state-sub">{t("catalogue.noResultsHint")}</span>
        </div>
      {/if}
    {/if}
  </div>
</div>

<CatalogueSongMenu
  bind:this={songMenu}
  onNavigateAlbum={(id) => onNavigate(`catalogue:album:${id}`)}
  onNavigateArtist={(id) => onNavigate(`catalogue:artist:${id}`)}
/>

<style>
  .cat-page {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .page-hdr {
    flex-shrink: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    padding: 22px 0 12px;
  }

  .hdr-copy {
    min-width: 0;
  }

  .kicker {
    font-size: 10px;
    letter-spacing: 0.16em;
    color: var(--faint);
    text-transform: uppercase;
  }

  .title {
    font-size: 38px;
    line-height: 1.05;
    letter-spacing: -0.02em;
    margin-top: 4px;
  }

  .hdr-row {
    display: flex;
    align-items: center;
    gap: 18px;
    flex-shrink: 0;
    margin-left: auto;
    padding-right: 10px;
  }

  .page-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding-bottom: 36px;
  }

  .page-scroll::-webkit-scrollbar {
    width: 6px;
  }

  .page-scroll::-webkit-scrollbar-thumb {
    background: rgba(240, 240, 236, 0.12);
    border-radius: 3px;
  }

  .page-scroll::-webkit-scrollbar-thumb:hover {
    background: rgba(240, 240, 236, 0.22);
  }

  .page-scroll::-webkit-scrollbar-track {
    background: transparent;
  }

  .section {
    margin-bottom: 26px;
  }

  .section-label {
    font-size: 13px;
    color: var(--dim);
    margin-bottom: 10px;
  }

  .fresh {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    min-height: 320px;
    margin-top: 6px;
    padding: 42px 20px;
    text-align: center;
    color: var(--faint);
  }

  .mix-note {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 18px;
    padding: 11px 14px;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--faint);
    font-size: 11.5px;
  }

  .mix-note .retry-btn {
    margin-left: auto;
    margin-top: 0;
  }

  .play-btn {
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
    transition:
      transform 0.15s,
      background 0.15s;
  }

  .play-btn :global(svg) {
    transform: rotate(-45deg);
  }

  .play-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 82%, #ffffff);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .play-btn:active:not(:disabled) {
    transform: rotate(45deg) scale(0.92);
  }

  .play-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .shuffle-btn {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line-strong);
    background: var(--bg-raise);
    color: var(--dim);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transform: rotate(45deg);
    transition:
      color 0.15s,
      border-color 0.15s,
      background 0.15s;
  }

  .shuffle-btn :global(svg) {
    transform: rotate(-45deg);
  }

  .shuffle-btn:hover:not(:disabled) {
    color: var(--text);
    background: var(--surface);
  }

  .shuffle-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .card-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(128px, 158px));
    gap: 10px;
  }

  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    min-height: 280px;
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
    max-width: 380px;
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
    .page-hdr {
      padding-top: 14px;
    }

    .title {
      font-size: 30px;
    }
  }

  @container player (max-width: 420px) {
    .title {
      font-size: 25px;
    }
  }
</style>
