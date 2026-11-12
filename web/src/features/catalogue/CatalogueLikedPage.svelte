<script lang="ts">
  import {
    Heart,
    Play,
    Shuffle,
    ListMusic,
    Clock,
    ListPlus,
    X,
  } from "lucide-svelte";
  import type { CatalogueTrack, Page } from "@lib/types";
  import {
    cataloguePath,
    refreshAfterCatalogueSave,
    resetCatalogueTaste,
    saveCatalogueTrack,
  } from "@state/catalogue.svelte";
  import {
    addToQueue,
    addToQueueFront,
    getCurrentTrack,
    matchesCurrentSong,
    requestPlay,
  } from "@state/now-playing.svelte";
  import { setShuffle } from "@state/player.svelte";
  import { getLikedTracks, toggleTasteLike } from "@state/taste.svelte";
  import { getCurrentPlaylists, saveStreamToPlaylist } from "@state/playlists.svelte";
  import {
    fmtTime,
    fmtTotalDuration,
    matchesQuery,
    toPascalCase,
    totalSeconds,
  } from "@lib/format";
  import { t } from "@lib/i18n";
  import { toasts } from "@lib/toast";
  import SearchInput from "@components/SearchInput.svelte";
  import TrackCover from "@components/TrackCover.svelte";
  import EqualizerBars from "@components/EqualizerBars.svelte";
  import ContextMenu, { type CtxEntry } from "@components/ContextMenu.svelte";
  import PlaylistPicker from "@features/vault/PlaylistPicker.svelte";

  let {
    onNavigate,
  }: {
    onNavigate: (page: Page) => void;
  } = $props();

  let q = $state("");
  let ctx = $state<{ x: number; y: number; track: CatalogueTrack } | null>(null);
  let pickerTarget = $state<CatalogueTrack | null>(null);
  let pickerBusy = $state(false);
  let streamTarget = $state<CatalogueTrack | null>(null);
  let streamBusy = $state(false);

  const tracks = $derived(getLikedTracks());
  const totalDur = $derived(totalSeconds(tracks));
  const currentTrackId = $derived(getCurrentTrack()?.id);
  const playlists = $derived(getCurrentPlaylists());
  const needle = $derived(q.toLowerCase());
  const hits = $derived(
    needle
      ? tracks.filter((track) =>
          matchesQuery(
            needle,
            `${track.title} ${track.artist} ${track.album?.name ?? ""}`.toLowerCase(),
          ),
        )
      : tracks,
  );

  function isCurrent(track: CatalogueTrack): boolean {
    if (track.id === currentTrackId) return true;
    return matchesCurrentSong(
      track.title,
      track.artist,
      track.duration ?? 0,
    );
  }

  function playAll(shuffle = false): void {
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

  function showCtx(e: MouseEvent, track: CatalogueTrack): void {
    e.preventDefault();
    e.stopPropagation();
    ctx = { x: e.clientX, y: e.clientY, track };
  }

  function ctxItems(): CtxEntry[] {
    if (!ctx) return [];
    const track = ctx.track;
    const items: CtxEntry[] = [
      {
        label: t("player.playNext"),
        icon: ListPlus,
        action: () => addToQueueFront(cataloguePath(track.id)),
      },
      {
        label: t("player.addToQueue"),
        icon: ListPlus,
        action: () => addToQueue(cataloguePath(track.id)),
      },
      {
        label: t("catalogue.addToPlaylist"),
        icon: ListPlus,
        action: () => {
          streamTarget = track;
        },
      },
      {
        label: t("catalogue.unlikeTrack"),
        icon: X,
        danger: true,
        action: () => {
          toggleTasteLike(track);
          resetCatalogueTaste();
        },
      },
      { type: "separator" },
      {
        label: t("catalogue.downloadToPlaylist"),
        icon: ListMusic,
        action: () => {
          pickerTarget = track;
        },
      },
    ];

    const albumId = track.album?.id ?? null;
    const artistId = track.artists.find((artist) => artist.id)?.id ?? null;
    if (albumId || artistId) items.push({ type: "separator" });
    if (albumId) {
      items.push({
        label: t("catalogue.goToAlbum"),
        action: () => onNavigate(`catalogue:album:${albumId}`),
      });
    }
    if (artistId) {
      items.push({
        label: t("catalogue.goToArtist"),
        action: () => onNavigate(`catalogue:artist:${artistId}`),
      });
    }
    return items;
  }

  async function addToSelectedPlaylist(playlistId: string): Promise<void> {
    const target = pickerTarget;
    if (!target || pickerBusy) return;
    pickerBusy = true;
    try {
      const result = await saveCatalogueTrack(target, playlistId);
      await refreshAfterCatalogueSave();
      const name =
        result.playlist?.name ??
        playlists.find((playlist) => playlist.id === playlistId)?.name ??
        "";
      toasts.success(
        t("catalogue.addedToPlaylist", { name: toPascalCase(name) }),
      );
    } catch (error) {
      console.warn("catalogue add to playlist failed:", error);
      toasts.error(t("catalogue.saveFailed"));
    } finally {
      pickerBusy = false;
      pickerTarget = null;
    }
  }

  async function createAndAddPlaylist(name: string): Promise<void> {
    const target = pickerTarget;
    if (!target || pickerBusy) return;
    pickerBusy = true;
    try {
      const result = await saveCatalogueTrack(target, null, name);
      await refreshAfterCatalogueSave();
      const playlistName = result.playlist?.name ?? name;
      toasts.success(
        t("catalogue.addedToPlaylist", { name: toPascalCase(playlistName) }),
      );
    } catch (error) {
      console.warn("catalogue create playlist failed:", error);
      toasts.error(t("catalogue.saveFailed"));
    } finally {
      pickerBusy = false;
      pickerTarget = null;
    }
  }

  async function addToExistingMix(mixId: string): Promise<void> {
    const target = streamTarget;
    if (!target || streamBusy) return;
    streamBusy = true;
    try {
      await saveStreamToPlaylist(target, mixId);
      const name =
        playlists.find((playlist) => playlist.id === mixId)?.name ?? "";
      toasts.success(
        t("catalogue.savedToPlaylist", { name: toPascalCase(name) }),
      );
    } catch (error) {
      console.warn("catalogue save to playlist failed:", error);
      toasts.error(t("catalogue.saveFailed"));
    } finally {
      streamBusy = false;
      streamTarget = null;
    }
  }

  async function createAndAddMix(name: string): Promise<void> {
    const target = streamTarget;
    if (!target || streamBusy) return;
    streamBusy = true;
    try {
      const playlist = await saveStreamToPlaylist(target, null, name);
      toasts.success(
        t("catalogue.savedToPlaylist", {
          name: toPascalCase(playlist.name),
        }),
      );
    } catch (error) {
      console.warn("catalogue create playlist failed:", error);
      toasts.error(t("catalogue.saveFailed"));
    } finally {
      streamBusy = false;
      streamTarget = null;
    }
  }
</script>

<div class="pl-detail">
  <div class="hero">
    <div class="hero-cover-wrap">
      <div class="hero-cover placeholder">
        <Heart size={22} stroke-width={1} />
      </div>
    </div>

    <div class="hero-meta">
      <div class="hero-name display" aria-label={t("catalogue.likedSongs")}>
        {t("catalogue.likedSongs")}
      </div>
      <div class="hero-sub mono">
        {#if tracks.length > 0}
          {tracks.length}
          {tracks.length === 1 ? t("library.song") : t("library.songsPlural")} ·
          {t("playlist.about")}
          {fmtTotalDuration(totalDur)}
        {:else}
          {t("playlist.zeroSongs")}
        {/if}
      </div>
    </div>

    <div class="hero-actions">
      <SearchInput
        value={q}
        placeholder={t("playlist.searchPlaceholder")}
        oninput={(v) => (q = v)}
      />
      <button
        class="play-big"
        onclick={() => playAll(false)}
        disabled={tracks.length === 0}
        aria-label={t("player.play")}
        title={t("catalogue.playAll")}
      >
        <Play size={16} fill="currentColor" stroke-width={0} />
      </button>
      <button
        class="icon-btn round"
        onclick={() => playAll(true)}
        disabled={tracks.length === 0}
        aria-label={t("player.shuffle")}
        title={t("catalogue.shuffle")}
      >
        <Shuffle size={14} stroke-width={1.5} />
      </button>
    </div>
  </div>

  <div class="tracks">
    <div class="tbl-head">
      <span class="h-num mono">#</span>
      <span class="h-cover"></span>
      <span class="h-title mono">{t("library.titleHeader")}</span>
      <span class="h-artist mono">{t("library.artistHeader")}</span>
      <span class="h-dur"><Clock size={12} stroke-width={2} /></span>
    </div>

    {#if hits.length === 0}
      <div class="tbl-empty">
        <ListMusic size={30} stroke-width={1.1} />
        <span class="display">
          {tracks.length === 0
            ? t("catalogue.likedEmptyTitle")
            : t("library.noMatches")}
        </span>
        <span class="mono sub">
          {tracks.length === 0
            ? t("catalogue.likedEmptyHint")
            : t("library.noMatchesQuery", { query: q })}
        </span>
      </div>
    {:else}
      {#each hits as track, i (track.id)}
        <div
          class="row"
          class:playing={isCurrent(track)}
          role="button"
          tabindex="0"
          aria-label={`${track.title} — ${track.artist}`}
          onclick={() => playTrack(track)}
          onkeydown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              playTrack(track);
            }
          }}
          oncontextmenu={(e) => showCtx(e, track)}
        >
          <span class="c-num mono">
            {#if isCurrent(track)}
              <EqualizerBars />
            {:else}
              {String(i + 1).padStart(2, "0")}
            {/if}
          </span>
          <div class="c-cover">
            <TrackCover src={track.cover} shadow="var(--shadow-sm)" />
          </div>
          <div class="c-title-col">
            <span class="c-title">{track.title}</span>
            {#if track.album}
              <span class="c-album">{track.album.name}</span>
            {/if}
          </div>
          <div class="c-artists">
            {#each track.artists as artist, idx (artist.id ?? `${artist.name}:${idx}`)}
              {#if idx > 0}<span class="comma">, </span>{/if}
              {#if artist.id}
                <button
                  type="button"
                  class="artist-link"
                  onclick={(e) => {
                    e.stopPropagation();
                    onNavigate(`catalogue:artist:${artist.id ?? ""}`);
                  }}
                >
                  {artist.name}
                </button>
              {:else}
                <span>{artist.name}</span>
              {/if}
            {/each}
          </div>
          <span class="c-dur mono">{fmtTime(track.duration, "--:--")}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>

{#if ctx}
  <ContextMenu
    x={ctx.x}
    y={ctx.y}
    items={ctxItems()}
    onclose={() => (ctx = null)}
  />
{/if}

{#if pickerTarget}
  <PlaylistPicker
    suggestedName={pickerTarget.album?.name ?? pickerTarget.title}
    trackCount={1}
    {playlists}
    busy={pickerBusy}
    onAddExisting={(id) => void addToSelectedPlaylist(id)}
    onCreate={(name) => void createAndAddPlaylist(name)}
    onSkip={() => {
      if (pickerBusy) return;
      pickerTarget = null;
    }}
  />
{/if}

{#if streamTarget}
  <PlaylistPicker
    suggestedName={streamTarget.album?.name ?? streamTarget.title}
    trackCount={1}
    {playlists}
    busy={streamBusy}
    onAddExisting={(id) => void addToExistingMix(id)}
    onCreate={(name) => void createAndAddMix(name)}
    onSkip={() => {
      if (streamBusy) return;
      streamTarget = null;
    }}
  />
{/if}

<style>
  .pl-detail {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
  }

  .hero {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 18px;
    padding: 20px 2px 12px;
  }

  .hero-cover-wrap {
    position: relative;
    flex-shrink: 0;
    --hero-cover: 128px;
    width: var(--hero-cover);
    height: var(--hero-cover);
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg-raise);
    box-shadow:
      0 0 0 1px rgba(255, 255, 255, 0.07),
      var(--shadow-md);
  }

  .hero-cover {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: cover;
  }

  .hero-cover.placeholder {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    background: var(--bg-raise);
  }

  .hero-cover.placeholder::before {
    content: "";
    position: absolute;
    inset: -40%;
    background:
      radial-gradient(
        circle at 50% 50%,
        color-mix(in srgb, var(--accent) 42%, transparent) 0%,
        transparent 62%
      );
    filter: blur(34px);
    opacity: 0.8;
  }

  .hero-cover.placeholder :global(svg) {
    position: relative;
    z-index: 1;
    filter: drop-shadow(0 2px 6px rgba(0, 0, 0, 0.55));
  }

  .hero-meta {
    flex: 1;
    min-width: 0;
  }

  .hero-name {
    width: 100%;
    font-size: 32px;
    line-height: 1.05;
    letter-spacing: -0.015em;
    color: var(--text);
    padding: 0 0 3px;
    margin-bottom: 6px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .hero-sub {
    font-size: 11.5px;
    color: var(--dim);
  }

  .hero-actions {
    display: flex;
    align-items: center;
    gap: 18px;
    align-self: flex-end;
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
    transition:
      transform 0.15s,
      background 0.15s;
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
    transition:
      color 0.15s,
      border-color 0.15s,
      background 0.15s;
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

  .tracks {
    --cover-size: 40px;
    --track-cols: 28px var(--cover-size) minmax(0, 1fr) minmax(0, 1fr) 42px;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding-right: 2px;
    padding-bottom: 8px;
  }

  .tracks::-webkit-scrollbar {
    width: 6px;
  }

  .tracks::-webkit-scrollbar-thumb {
    background: rgba(240, 240, 236, 0.12);
    border-radius: 3px;
  }

  .tracks::-webkit-scrollbar-track {
    background: transparent;
  }

  .tbl-head {
    display: grid;
    grid-template-columns: var(--track-cols);
    align-items: center;
    gap: 12px;
    padding: 0 10px 8px;
    position: sticky;
    top: 0;
    z-index: 2;
    background: var(--bg);
    color: var(--text);
    font-size: 13px;
  }

  .h-num {
    text-align: right;
    font-size: 11px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }

  .h-title,
  .h-artist {
    min-width: 0;
    letter-spacing: 0.14em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .h-dur {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    color: var(--text);
  }

  .h-dur :global(svg) {
    display: block;
    flex-shrink: 0;
  }

  .c-num {
    font-size: 11px;
    color: var(--faint);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .c-cover {
    display: flex;
  }

  .c-title-col {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .c-title {
    font-size: 13px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.2;
  }

  .c-album {
    font-size: 10.5px;
    color: var(--faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .c-artists {
    display: flex;
    flex-wrap: nowrap;
    align-items: baseline;
    min-width: 0;
    overflow: hidden;
    font-size: 12px;
    color: var(--dim);
    white-space: nowrap;
  }

  .c-artists .comma {
    flex-shrink: 0;
  }

  .artist-link {
    background: none;
    border: none;
    padding: 0;
    color: inherit;
    font: inherit;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
    transition: color 0.12s;
  }

  .artist-link:hover {
    color: var(--text);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .c-dur {
    font-size: 11px;
    color: var(--faint);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .row {
    position: relative;
    display: grid;
    grid-template-columns: var(--track-cols);
    align-items: center;
    gap: 12px;
    padding: 7px 10px;
    border-radius: var(--radius-sm);
    transition: background 0.12s;
    user-select: none;
    cursor: pointer;
  }

  .row:hover {
    background: var(--bg-raise);
  }

  .row.playing {
    background: var(--accent-soft);
  }

  .row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .tbl-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    color: var(--faint);
  }

  .tbl-empty .display {
    font-size: 26px;
    color: var(--dim);
  }

  .tbl-empty .sub {
    font-size: 10.5px;
    letter-spacing: 0.06em;
    text-align: center;
  }

  @container player (max-width: 760px) {
    .hero-cover-wrap {
      --hero-cover: 108px;
    }
  }

  @container player (max-width: 540px) {
    .hero {
      gap: 12px;
    }

    .hero-cover-wrap {
      --hero-cover: 92px;
    }

    .hero-name {
      font-size: 26px;
    }
  }
</style>
