<script lang="ts">
  import {
    ListMusic,
    ListPlus,
    Pause,
    Play,
    Clock,
  } from "lucide-svelte";
  import type { CatalogueTrack } from "@lib/types";
  import {
    addToQueue,
    addToQueueFront,
    getCurrentTrack,
    getIsTrackPlaying,
    matchesCurrentSong,
  } from "@state/now-playing.svelte";
  import { toggle } from "@lib/player.svelte";
  import {
    getCurrentPlaylists,
    saveStreamToPlaylist,
  } from "@state/playlists.svelte";
  import {
    cataloguePath,
    refreshAfterCatalogueSave,
    saveCatalogueTrack,
  } from "@state/catalogue.svelte";
  import { fmtTime, toPascalCase } from "@lib/format";
  import { t } from "@lib/i18n";
  import { toasts } from "@lib/toast";
  import TrackCover from "@components/TrackCover.svelte";
  import EqualizerBars from "@components/EqualizerBars.svelte";
  import ContextMenu, { type CtxEntry } from "@components/ContextMenu.svelte";
  import PlaylistPicker from "@features/vault/PlaylistPicker.svelte";

  let {
    tracks,
    showAlbum = true,
    emptyTitle = "",
    emptyHint = "",
    onPlayTrack,
    onNavigateAlbum,
    onNavigateArtist,
  }: {
    tracks: readonly CatalogueTrack[];
    showAlbum?: boolean;
    emptyTitle?: string;
    emptyHint?: string;
    onPlayTrack: (track: CatalogueTrack) => void;
    onNavigateAlbum?: (id: string) => void;
    onNavigateArtist?: (id: string) => void;
  } = $props();

  const nowPlaying = $derived(getCurrentTrack());
  const isPlaying = $derived(getIsTrackPlaying());
  const playlists = $derived(getCurrentPlaylists());

  let ctx = $state<{ x: number; y: number; track: CatalogueTrack } | null>(null);
  let pickerTarget = $state<CatalogueTrack | null>(null);
  let pickerBusy = $state(false);
  let streamTarget = $state<CatalogueTrack | null>(null);
  let streamBusy = $state(false);

  function rowAction(track: CatalogueTrack): void {
    if (nowPlaying?.id === track.id) {
      toggle();
    } else {
      onPlayTrack(track);
    }
  }

  function isCurrent(track: CatalogueTrack): boolean {
    if (nowPlaying?.id === track.id) return true;
    return matchesCurrentSong(
      track.title,
      track.artist,
      track.duration ?? 0,
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
    const isCurrent = nowPlaying?.id === track.id && isPlaying;
    const items: CtxEntry[] = [
      {
        label: isCurrent ? t("contextMenu.pause") : t("contextMenu.play"),
        icon: isCurrent ? Pause : Play,
        action: () => rowAction(track),
      },
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
    if ((albumId && onNavigateAlbum) || (artistId && onNavigateArtist)) {
      items.push({ type: "separator" });
    }
    if (albumId && onNavigateAlbum) {
      items.push({
        label: t("catalogue.goToAlbum"),
        action: () => onNavigateAlbum(albumId),
      });
    }
    if (artistId && onNavigateArtist) {
      items.push({
        label: t("catalogue.goToArtist"),
        action: () => onNavigateArtist(artistId),
      });
    }
    return items;
  }

  function closePicker(): void {
    if (pickerBusy) return;
    pickerTarget = null;
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
        t("catalogue.addedToPlaylist", {
          name: toPascalCase(playlistName),
        }),
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

<div class="track-list">
  <div class="tbl-head">
    <span class="h-num mono">#</span>
    <span class="h-cover"></span>
    <span class="h-title mono">{t("library.titleHeader")}</span>
    <span class="h-artist mono">{t("library.artistHeader")}</span>
    <span class="h-dur"><Clock size={12} stroke-width={2} /></span>
  </div>

  {#if tracks.length === 0}
    <div class="tbl-empty">
      <ListMusic size={28} stroke-width={1.1} />
      <span class="empty-title display">{emptyTitle || t("catalogue.noResults")}</span>
      {#if emptyHint}
        <span class="empty-sub mono">{emptyHint}</span>
      {/if}
    </div>
  {:else}
    {#each tracks as track, i (track.id)}
      <div
        class="row"
        class:playing={isCurrent(track)}
        role="button"
        tabindex="0"
        aria-label={`${track.title} - ${track.artist}`}
        onclick={() => rowAction(track)}
        onkeydown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            rowAction(track);
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
        <div class="c-meta">
          <span class="c-title" class:accent={isCurrent(track)}>
            {track.title}
          </span>
          {#if showAlbum && track.album}
            <span class="c-album">{track.album.name}</span>
          {/if}
        </div>
        <div class="c-artists">
          {#each track.artists as artist, idx (artist.id ?? `${artist.name}:${idx}`)}
            {#if idx > 0}<span class="comma">, </span>{/if}
            {#if artist.id && onNavigateArtist}
              <button
                type="button"
                class="artist-link"
                onclick={(e) => {
                  e.stopPropagation();
                  onNavigateArtist?.(artist.id ?? "");
                }}
              >
                {artist.name}
              </button>
            {:else}
              <span>{artist.name}</span>
            {/if}
          {/each}
        </div>
        <div class="c-end">
          <span class="c-dur mono">{fmtTime(track.duration, "--:--")}</span>
        </div>
      </div>
    {/each}
  {/if}
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
    onSkip={closePicker}
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
  .track-list {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .tbl-head {
    display: grid;
    grid-template-columns: 28px 40px minmax(0, 1.4fr) minmax(0, 1fr) 64px;
    align-items: center;
    gap: 12px;
    padding: 8px 10px;
    position: sticky;
    top: 0;
    z-index: 3;
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

  .row {
    display: grid;
    grid-template-columns: 28px 40px minmax(0, 1.4fr) minmax(0, 1fr) 64px;
    align-items: center;
    gap: 12px;
    padding: 6px 10px;
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

  .c-num {
    font-size: 11px;
    color: var(--faint);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .c-cover {
    display: flex;
  }

  .c-meta {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .c-title {
    font-size: 13px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.2;
  }

  .c-title.accent {
    color: var(--accent);
  }

  .c-album {
    font-size: 10.5px;
    color: var(--faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
    padding: 0;
    border: none;
    background: none;
    text-align: left;
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

  .c-end {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    min-width: 0;
  }

  .tbl-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 46px 16px;
    color: var(--faint);
  }

  .empty-title {
    font-size: 20px;
    color: var(--dim);
  }

  .empty-sub {
    font-size: 10.5px;
    letter-spacing: 0.04em;
    text-align: center;
  }

  @container player (max-width: 760px) {
    .tbl-head,
    .row {
      grid-template-columns: 24px 36px minmax(0, 1.4fr) minmax(0, 1fr) 58px;
      gap: 8px;
    }
  }

  @container player (max-width: 540px) {
    .tbl-head,
    .row {
      grid-template-columns: 20px minmax(0, 1.5fr) minmax(0, 1fr) 54px;
      gap: 6px;
    }

    .h-cover,
    .c-cover {
      display: none;
    }
  }

  @container player (max-width: 420px) {
    .tbl-head,
    .row {
      grid-template-columns: 18px minmax(0, 1fr) 34px;
      padding-left: 6px;
      padding-right: 6px;
    }

    .h-artist,
    .c-artists {
      display: none;
    }

    .c-end {
      gap: 0;
    }
  }
</style>
