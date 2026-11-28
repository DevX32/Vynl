<script lang="ts">
  import { untrack } from "svelte";
  import { fade, fly } from "svelte/transition";
  import { cubicIn, cubicOut } from "svelte/easing";
  import {
    Play,
    Shuffle,
    Plus,
    X,
    ListMusic,
    Disc3,
    ListPlus,
    Image,
    ImageOff,
    Clock,
    LoaderCircle,
    Pencil,
    Trash2,
  } from "lucide-svelte";
  import {
    getCurrentTrack,
    matchesCurrentSong,
    requestPlay,
    addToQueue,
  } from "@state/now-playing.svelte";
  import { getLibrary } from "@state/library.svelte";
  import { setShuffle } from "@state/player.svelte";
  import {
    registerCatalogueTracks,
    saveCatalogueTrack,
    refreshAfterCatalogueSave,
  } from "@state/catalogue.svelte";
  import {
    fmtTotalDuration,
    fmtTime,
    totalSeconds,
    matchesQuery,
  } from "../../lib/format";
  import { blobSrc } from "../../lib/format";
  import { useDragList } from "@lib/drag-list.svelte";
  import { normalizeCoverImage } from "@lib/cover-image";
  import { toasts } from "@lib/toast";
  import { vynl } from "../../lib/vynl";
  import SearchInput from "../../components/SearchInput.svelte";
  import TrackCover from "../../components/TrackCover.svelte";
  import EqualizerBars from "../../components/EqualizerBars.svelte";
  import ContextMenu, {
    type CtxEntry,
  } from "../../components/ContextMenu.svelte";
  import Dialog from "../../components/Dialog.svelte";
  import VirtualList from "../../components/VirtualList.svelte";
  import {
    addToPlaylist,
    deletePlaylist,
    getCurrentPlaylist,
    isCataloguePath,
    isPlaylistLoading,
    markCoverFailed,
    moveInPlaylist,
    removeFromPlaylist,
    renamePlaylist,
    resolvePlaylistCovers,
    selectPlaylist,
    setPlaylistCover,
    tracksOf,
  } from "@state/playlists.svelte";
  import { t, tp } from "@lib/i18n";
  import { focusTrap } from "@lib/actions/focus-trap";
  import BulkTracksOverlay from "@components/BulkTracksOverlay.svelte";

  let q = $state("");
  let ctxMenu = $state<{ x: number; y: number; idx: number } | null>(null);
  let removeTarget = $state<(typeof plHits)[number] | null>(null);
  let removeBusy = $state(false);
  let lastPlaylistId: string | undefined;

  const pl = $derived(getCurrentPlaylist());
  const library = $derived(getLibrary());
  const playlistStreams = $derived(pl?.streams ?? []);
  const loading = $derived(isPlaylistLoading());

  $effect(() => {
    const id = pl?.id;
    if (id !== lastPlaylistId) {
      lastPlaylistId = id;
      untrack(() => {
        if (q !== "") q = "";
      });
    }
    const streams = pl?.streams;
    if (streams && streams.length > 0) {
      const id = window.setTimeout(() => registerCatalogueTracks(streams), 0);
      return () => window.clearTimeout(id);
    }
  });

  const currentTracks = $derived(pl ? tracksOf(pl) : []);
  const existingPaths = $derived(
    new Set(currentTracks.map((track) => track.path)),
  );
  const addableLibrary = $derived(
    library.filter((track) => !existingPaths.has(track.path)),
  );
  const covers = $derived(resolvePlaylistCovers(pl?.cover, currentTracks));
  const needle = $derived(q.toLowerCase());
  const plHits = $derived(
    needle
      ? currentTracks.filter((t) =>
          matchesQuery(
            needle,
            `${t.title} ${t.artist} ${t.album}`.toLowerCase(),
          ),
        )
      : currentTracks,
  );

  const currentTrackId = $derived(getCurrentTrack()?.id);

  function isCurrent(track: {
    id: string;
    title: string;
    artist: string;
    duration: number;
  }): boolean {
    if (track.id === currentTrackId) return true;
    return matchesCurrentSong(track.title, track.artist, track.duration);
  }

  const { drag, start, shouldSkipClick } = useDragList({
    pathAt: (_section, idx) => plHits[idx]?.path,
    onReorder: (fromPath, toPath) => void moveTrack(fromPath, toPath),
  });

  let adding = $state(false);
  let downloadBusy = $state(false);
  let bulkMode = $state<"add" | "download">("add");
  let bulkDone = $state(0);
  let bulkTotal = $state(0);
  let coverInput: HTMLInputElement | undefined = $state();
  let coverBusy = $state(false);
  let tracksEl: HTMLDivElement | undefined = $state();
  let editOpen = $state(false);
  let editRename = $state("");
  let editBusy = $state(false);
  let confirmDeletePlaylistOpen = $state(false);

  const totalDur = $derived(totalSeconds(currentTracks));

  function playAll(shuffle = false): void {
    if (currentTracks.length === 0 || !pl) return;
    if (shuffle) setShuffle(true);
    const first = shuffle
      ? currentTracks[Math.floor(Math.random() * currentTracks.length)]
      : currentTracks[0];
    requestPlay(
      first.id,
      currentTracks.map((track) => track.path),
    );
  }

  async function addPaths(paths: string[]): Promise<void> {
    if (!pl || downloadBusy || paths.length === 0) return;
    downloadBusy = true;
    bulkMode = "add";
    bulkTotal = paths.length;
    bulkDone = paths.length;
    try {
      await addToPlaylist(pl.id, paths);
      adding = false;
      toasts.success(
        tp("vault.addedToPlaylist", paths.length, {
          n: paths.length,
          name: pl.name,
        }),
      );
    } catch (e) {
      console.warn("addToPlaylist error:", e);
      toasts.error(t("playlist.addFailed"));
    } finally {
      downloadBusy = false;
      bulkDone = 0;
      bulkTotal = 0;
    }
  }

  async function downloadTracks(
    tracks: {
      id: string;
      title: string;
      artist: string;
      artists: { name: string; id: string | null }[];
      album: { name: string; id: string | null } | null;
      duration: number | null;
      cover: string | null;
      explicit: boolean;
    }[],
  ): Promise<void> {
    if (!pl || downloadBusy || tracks.length === 0) return;
    downloadBusy = true;
    bulkMode = "download";
    bulkTotal = tracks.length;
    bulkDone = 0;
    let done = 0;
    let failed = 0;
    try {
      for (const track of tracks) {
        try {
          await saveCatalogueTrack(track, pl.id);
          done += 1;
        } catch (error) {
          failed += 1;
          console.warn("download failed for", track.id, error);
        }
        bulkDone = done;
      }
      try {
        await refreshAfterCatalogueSave();
      } catch (error) {
        console.warn("download refresh failed:", error);
      }
      adding = false;
      if (failed === 0) {
        toasts.success(
          t("catalogue.downloadDone", { n: done, name: pl.name }),
        );
      } else {
        toasts.error(t("catalogue.downloadPartial", { done, failed }));
      }
    } finally {
      downloadBusy = false;
      bulkDone = 0;
      bulkTotal = 0;
    }
  }

  async function moveTrack(fromPath: string, toPath: string): Promise<void> {
    if (!pl) return;
    const from = pl.paths.indexOf(fromPath);
    const to = pl.paths.indexOf(toPath);
    if (from === to || from < 0 || to < 0) return;
    try {
      await moveInPlaylist(pl.id, from, to);
    } catch (e) {
      console.warn("moveInPlaylist error:", e);
      toasts.error(t("playlist.reorderFailed"));
    }
  }

  async function confirmRemoveTrack(): Promise<void> {
    const target = removeTarget;
    if (!target || !pl || removeBusy) return;
    removeBusy = true;
    const playlistName = pl.name;
    try {
      await removeFromPlaylist(pl.id, target.path);
      toasts.success(
        t("playlist.removedTrack", {
          title: target.title,
          name: playlistName,
        }),
      );
    } catch (e) {
      console.warn("removeFromPlaylist error:", e);
      toasts.error(t("playlist.removeTrackFailed"));
    } finally {
      removeBusy = false;
      removeTarget = null;
    }
  }

  function showTrackCtx(e: MouseEvent, idx: number): void {
    e.preventDefault();
    if (downloadBusy) return;
    ctxMenu = { x: e.clientX, y: e.clientY, idx };
  }

  function trackCtxItems(): CtxEntry[] {
    if (!ctxMenu || !pl) return [];
    const trk = plHits[ctxMenu.idx];
    if (!trk) return [];
    const items: CtxEntry[] = [
      {
        label: t("common.addToQueue"),
        icon: ListPlus,
        action: () => addToQueue(trk.path),
      },
    ];
    if (!downloadBusy) {
      items.push(
        { type: "separator" },
        {
          label: t("playlist.remove"),
          icon: X,
          danger: true,
          action: () => {
            removeTarget = trk;
          },
        },
      );
    }
    return items;
  }

  function onRowPointerDown(e: PointerEvent, idx: number): void {
    if (downloadBusy || !!q) return;
    const track = plHits[idx];
    if (track) {
      start(e, { idx, path: track.path, paths: plHits.map((t) => t.path) });
    }
  }

  function pickCover(): void {
    coverInput?.click();
  }

  async function onCoverFile(e: Event): Promise<void> {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file || !pl || coverBusy) return;
    coverBusy = true;
    try {
      const { ext, data } = await normalizeCoverImage(file);
      const savedPath = await vynl.savePlaylistCover(pl.id, ext, data);
      await setPlaylistCover(pl.id, savedPath);
    } catch (err) {
      console.warn("set playlist cover failed:", err);
      toasts.error(t("playlist.coverFailed"));
    } finally {
      coverBusy = false;
    }
  }

  function openEditModal(): void {
    if (!pl) return;
    editRename = pl.name;
    confirmDeletePlaylistOpen = false;
    editOpen = true;
  }

  async function savePlaylistRename(): Promise<void> {
    if (!pl || editBusy) return;
    const next = editRename.trim();
    if (!next || next === pl.name) {
      editOpen = false;
      return;
    }
    editBusy = true;
    try {
      await renamePlaylist(pl.id, next);
      editOpen = false;
    } catch (err) {
      console.warn("rename playlist failed:", err);
      toasts.error(t("playlist.renameFailed"));
    } finally {
      editBusy = false;
    }
  }

  async function removeCover(): Promise<void> {
    if (!pl || coverBusy) return;
    coverBusy = true;
    try {
      await setPlaylistCover(pl.id, null);
    } catch (err) {
      console.warn("remove playlist cover failed:", err);
      toasts.error(t("playlist.coverFailed"));
    } finally {
      coverBusy = false;
    }
  }

  async function confirmDeletePlaylist(): Promise<void> {
    if (!pl || editBusy) return;
    editBusy = true;
    try {
      await deletePlaylist(pl.id);
      confirmDeletePlaylistOpen = false;
      editOpen = false;
      await selectPlaylist(null);
    } catch (err) {
      console.warn("delete playlist failed:", err);
      toasts.error(t("playlists.deleteFailed"));
    } finally {
      editBusy = false;
    }
  }
</script>

<div class="pl-detail">
  {#if loading && !pl}
    <div class="pl-loading" role="status">
      <LoaderCircle size={18} stroke-width={1.5} class="spin" />
      <span class="mono">{t("common.loading")}</span>
    </div>
  {:else if pl}
    <div class="hero">
      <div class="hero-cover-wrap">
        {#if covers.length >= 2}
          <div class="hero-cover-grid">
            {#each covers as c (c)}
              <img
                class="hero-cover-tile"
                use:blobSrc={c}
                alt=""
                loading="lazy"
                draggable="false"
                onerror={() => markCoverFailed(c)}
              />
            {/each}
            {#each Array(4 - covers.length) as _}
              <div class="hero-cover-tile hero-cover-tile-empty"></div>
            {/each}
          </div>
        {:else if covers.length === 1}
          <img
            class="hero-cover"
            use:blobSrc={covers[0]}
            alt=""
            loading="lazy"
            draggable="false"
            onerror={() => markCoverFailed(covers[0])}
          />
        {:else}
          <div class="hero-cover placeholder">
            <Disc3 size={22} stroke-width={1} />
          </div>
        {/if}
        <button
          class="cover-pick-btn"
          onclick={openEditModal}
          aria-label={t("playlist.editPlaylist")}
        >
          <Pencil size={14} stroke-width={1.5} />
        </button>
        <input
          bind:this={coverInput}
          type="file"
          accept="image/*"
          class="cover-file-input"
          onchange={onCoverFile}
        />
      </div>

      <div class="hero-meta">
        <div class="hero-name display" aria-label={t("common.playlistName")}>
          {pl.name}
        </div>
        <div class="hero-sub mono">
          {#if currentTracks.length > 0}
            {currentTracks.length}
            {currentTracks.length === 1 ? t("library.song") : t("common.songs")} ·
            {t("common.about")}
            {fmtTotalDuration(totalDur)}
          {:else}
            {t("playlist.zeroSongs")}
          {/if}
          {#if downloadBusy}
            · {bulkMode === "add"
              ? t("catalogue.adding", {
                  done: bulkDone,
                  total: bulkTotal,
                })
              : t("catalogue.downloading", {
                  done: bulkDone,
                  total: bulkTotal,
                })}
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
          disabled={currentTracks.length === 0}
          aria-label={t("common.play")}
        >
          <Play size={16} fill="currentColor" stroke-width={0} />
        </button>
        <button
          class="icon-btn round"
          onclick={() => playAll(true)}
          disabled={currentTracks.length === 0}
          aria-label={t("common.shuffle")}
        >
          <Shuffle size={14} stroke-width={1.5} />
        </button>
        <button
          class="icon-btn round"
          onclick={() => (adding = true)}
          disabled={downloadBusy}
          aria-label={t("playlist.addTracks")}
        >
          <Plus size={14} stroke-width={1.5} />
        </button>
      </div>
    </div>

    <div class="tbl-head">
      <span class="h-num mono">#</span>
      <span class="h-cover"></span>
      <span class="h-title mono">{t("common.titleLabel")}</span>
      <span class="h-artist mono">{t("common.artistLabel")}</span>
      <span class="h-dur"><Clock size={12} stroke-width={2} /></span>
    </div>

    <div class="tracks" bind:this={drag.rowsEl} bind:this={tracksEl}>
      {#if plHits.length === 0}
        <div class="tbl-empty">
          <ListMusic size={30} stroke-width={1.1} />
          <span class="display">
            {pl.paths.length === 0
              ? t("playlist.empty")
              : t("common.noMatches")}
          </span>
          <span class="mono sub">
            {pl.paths.length === 0
              ? t("playlist.addSomeTracks")
              : t("common.nothingMatchesQuery", { query: q })}
          </span>
        </div>
      {:else}
        {#snippet row(trk: (typeof plHits)[number], i: number)}
          <div
            class="row"
            class:playing={isCurrent(trk)}
            class:drag-over={drag.dragging &&
              drag.overIdx === i &&
              drag.grabIdx !== i}
            class:dragging={drag.dragging && drag.grabIdx === i}
            role="button"
            tabindex="0"
            aria-label={`${trk.title} — ${trk.artist}`}
            style:cursor={!q
              ? drag.dragging
                ? "grabbing"
                : "grab"
              : "pointer"}
            onpointerdown={(e) => onRowPointerDown(e, i)}
            oncontextmenu={(e) => showTrackCtx(e, i)}
            onclick={() => {
              if (shouldSkipClick()) return;
              requestPlay(trk.id, pl.paths);
            }}
            onkeydown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                requestPlay(trk.id, pl.paths);
              }
            }}
          >
            <span class="c-num mono">
              {#if isCurrent(trk)}
                <EqualizerBars />
              {:else}
                {String(i + 1).padStart(2, "0")}
              {/if}
            </span>
            <div class="c-cover">
              <TrackCover src={trk.cover} shadow="var(--shadow-sm)" />
            </div>
            <div class="c-title-col">
              <span class="c-title" class:accent={isCurrent(trk)}>
                {trk.title}
              </span>
              {#if trk.album}
                <span class="c-album">{trk.album}</span>
              {/if}
            </div>
            <span class="c-artist">{trk.artist}</span>
            <span class="c-dur mono">{fmtTime(trk.duration, "--:--")}</span>
          </div>
        {/snippet}

        <VirtualList
          items={plHits}
          {row}
          scrollEl={tracksEl ?? null}
          rowKey={(trk) => trk.id}
          estimateRowHeight={54}
        />
      {/if}
    </div>

    {#if adding}
      <BulkTracksOverlay
        addTracks={addableLibrary}
        downloadTracks={playlistStreams}
        busy={downloadBusy}
        onClose={() => (adding = false)}
        onSubmit={(picked, mode) => {
          if (mode === "add") {
            const paths = picked
              .map(
                (t) =>
                  (t as { path?: string }).path ??
                  library.find((l) => l.id === t.id)?.path,
              )
              .filter((p): p is string => !!p);
            void addPaths(paths);
          } else {
            void downloadTracks(picked as any);
          }
        }}
      />
    {/if}
  {:else}
    <div class="tbl-empty">
      <ListMusic size={30} stroke-width={1.1} />
      <span class="display">{t("playlists.playlistUnavailable")}</span>
    </div>
  {/if}

  {#if ctxMenu}
    <ContextMenu
      x={ctxMenu.x}
      y={ctxMenu.y}
      onClose={() => (ctxMenu = null)}
      items={trackCtxItems()}
    />
  {/if}

  <Dialog
    open={removeTarget !== null}
    title={t("playlist.removeTrackTitle")}
    description={removeTarget
      ? t(
          isCataloguePath(removeTarget.path)
            ? "playlist.removeStreamBody"
            : "playlist.removeTrackBody",
          {
            title: removeTarget.title,
          },
        )
      : ""}
    confirmLabel={t("common.delete")}
    danger
    onConfirm={() => void confirmRemoveTrack()}
    onClose={() => (removeTarget = null)}
  />

  {#if editOpen && pl}
    <div
      class="backdrop"
      role="presentation"
      transition:fade={{ duration: 150 }}
      onclick={() => (editOpen = false)}
    >
      <div
        class="edit-panel"
        role="dialog"
        aria-modal="true"
        aria-label={t("playlist.editPlaylist")}
        tabindex="-1"
        in:fly={{ y: 8, duration: 180, easing: cubicOut }}
        out:fly={{ y: 8, duration: 140, easing: cubicIn }}
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            editOpen = false;
          }
        }}
        use:focusTrap
      >
        <div class="edit-head">
          <span class="edit-title">{t("playlist.editPlaylist")}</span>
          <button
            class="edit-close"
            onclick={() => (editOpen = false)}
            aria-label={t("common.cancel")}
          >
            <X size={12} stroke-width={1.6} />
          </button>
        </div>

        <div class="edit-hero">
          <div class="edit-cover">
            {#if covers.length >= 2}
              <div class="edit-cover-grid">
                {#each covers as c (c)}
                  <img
                    class="edit-cover-tile"
                    use:blobSrc={c}
                    alt=""
                    loading="lazy"
                    draggable="false"
                  />
                {/each}
                {#each Array(4 - covers.length) as _}
                  <div class="edit-cover-tile edit-cover-empty"></div>
                {/each}
              </div>
            {:else if covers.length === 1}
              <img
                class="edit-cover-img"
                use:blobSrc={covers[0]}
                alt=""
                loading="lazy"
                draggable="false"
              />
            {:else}
              <div class="edit-cover-placeholder">
                <Disc3 size={24} stroke-width={1} />
              </div>
            {/if}
            <div class="edit-cover-overlay">
              <button
                class="cover-action"
                onclick={pickCover}
                disabled={coverBusy}
                aria-label={t("playlist.changeCover")}
              >
                <Image size={14} stroke-width={1.5} />
              </button>
              {#if pl.cover}
                <button
                  class="cover-action"
                  onclick={() => void removeCover()}
                  disabled={coverBusy}
                  aria-label={t("playlist.removeCover")}
                >
                  <ImageOff size={14} stroke-width={1.5} />
                </button>
              {/if}
            </div>
          </div>
          <div class="edit-field">
            <label class="edit-label" for="pl-rename">
              {t("playlists.renameTitle")}
            </label>
            <div class="edit-row">
              <input
                id="pl-rename"
                class="dlg-input"
                type="text"
                bind:value={editRename}
                spellcheck={false}
                onkeydown={(e) => {
                  if (e.key === "Enter") void savePlaylistRename();
                }}
              />
              <button
                class="action-btn-sm"
                onclick={() => void savePlaylistRename()}
                disabled={editBusy || !editRename.trim()}
              >
                {t("common.save")}
              </button>
            </div>
            <button
              class="edit-action"
              onclick={() => (confirmDeletePlaylistOpen = true)}
              disabled={editBusy}
            >
              <Trash2 size={14} stroke-width={1.5} />
              <span>{t("playlists.deleteTitle")}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <Dialog
    open={confirmDeletePlaylistOpen && !!pl}
    title={t("playlists.deleteTitle")}
    description={pl ? t("playlists.deleteBody", { name: pl.name }) : ""}
    confirmLabel={t("common.delete")}
    danger
    confirmDisabled={editBusy}
    onConfirm={() => void confirmDeletePlaylist()}
    onClose={() => {
      if (!editBusy) confirmDeletePlaylistOpen = false;
    }}
  />
</div>

<style>
  .pl-detail {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    --cover-size: 40px;
    --track-cols: 28px var(--cover-size) minmax(0, 1fr) minmax(0, 1fr) 42px;
  }

  .pl-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    flex: 1;
    min-height: 0;
    color: var(--faint);
    font-size: 11.5px;
  }

  .pl-loading :global(.spin) {
    display: inline-flex;
    align-items: center;
    animation: spin 0.85s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
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

  .hero-cover-grid {
    width: 100%;
    height: 100%;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0;
    background: var(--bg);
  }

  .hero-cover-tile {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    background: var(--bg-raise);
  }

  .hero-cover-tile-empty {
    background: var(--placeholder-gradient);
  }

  .hero-cover.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--faint);
    background: var(--placeholder-gradient);
  }

  .cover-pick-btn {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--overlay-bg);
    border: none;
    border-radius: 0;
    color: #fff;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .hero-cover-wrap:hover .cover-pick-btn,
  .cover-pick-btn:focus-visible {
    opacity: 1;
  }

  .cover-file-input {
    display: none;
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
    background: var(--bg-raise);
  }

  .icon-btn.round:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .tracks {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding-right: 2px;
    padding-bottom: 8px;
  }

  .tbl-head {
    grid-template-columns: var(--track-cols);
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

  .c-title.accent {
    color: var(--accent);
  }

  .c-album {
    font-size: 10.5px;
    color: var(--faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .c-artist {
    font-size: 12px;
    color: var(--dim);
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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

  .row.dragging {
    opacity: 0.3;
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
  }

  @container player (max-width: 760px) {
    .hero-cover-wrap {
      --hero-cover: 108px;
    }
  }

  .backdrop {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 16px;
    background: var(--overlay-bg);
    backdrop-filter: blur(8px);
  }

  .edit-panel {
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 16px;
    width: min(420px, 100%);
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow:
      0 24px 64px rgba(0, 0, 0, 0.55),
      inset 0 1px 0 rgba(255, 255, 255, 0.03);
  }

  .edit-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .edit-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text);
    letter-spacing: -0.01em;
  }

  .edit-close {
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line);
    color: var(--faint);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transform: rotate(45deg);
  }

  .edit-close:hover {
    color: var(--text);
    border-color: var(--line-strong);
  }

  .edit-close :global(svg) {
    transform: rotate(-45deg);
  }

  .edit-hero {
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .edit-cover {
    position: relative;
    width: 112px;
    height: 112px;
    flex: 0 0 112px;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg-raise);
    border: 1px solid var(--line);
  }

  .edit-cover-img,
  .edit-cover-placeholder {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .edit-cover-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--faint);
  }

  .edit-cover-grid {
    width: 100%;
    height: 100%;
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
  }

  .edit-cover-tile {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .edit-cover-empty {
    background: var(--bg-raise);
  }

  .edit-cover-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 8px;
    background: rgba(0, 0, 0, 0.42);
    opacity: 0;
    transition: opacity 0.14s ease;
  }

  .edit-cover:hover .edit-cover-overlay,
  .edit-cover:focus-within .edit-cover-overlay {
    opacity: 1;
  }

  @media (hover: none) {
    .edit-cover-overlay {
      opacity: 1;
    }
  }

  .cover-action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: var(--radius-sm);
    background: rgba(18, 18, 18, 0.5);
    color: #fff;
    cursor: pointer;
    backdrop-filter: blur(6px);
  }

  .cover-action:hover:not(:disabled) {
    background: rgba(18, 18, 18, 0.72);
  }

  .cover-action:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .edit-hero .edit-field {
    flex: 1;
    min-width: 0;
  }

  .edit-field {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .edit-label {
    font-size: 11px;
    color: var(--dim);
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .edit-row {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .edit-row .dlg-input {
    flex: 1;
    min-width: 0;
  }

  .action-btn-sm {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line-strong);
    background: var(--bg-raise);
    color: var(--text);
    font: 500 12px inherit;
    cursor: pointer;
    white-space: nowrap;
  }

  .action-btn-sm:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .edit-action {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
    background: transparent;
    color: var(--text);
    font: 500 12px inherit;
    cursor: pointer;
  }

  .edit-action:hover:not(:disabled) {
    background: var(--bg-raise);
  }

  .edit-action:disabled {
    opacity: 0.45;
    cursor: default;
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
