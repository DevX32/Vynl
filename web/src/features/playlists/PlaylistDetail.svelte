<script lang="ts">
  import {
    Play,
    Shuffle,
    Plus,
    X,
    ListMusic,
    Disc3,
    ListPlus,
    Image,
    Clock,
    Loader,
  } from "lucide-svelte";
  import {
    getCurrentTrack,
    requestPlay,
    addToQueue,
  } from "@state/now-playing.svelte";
  import { getLibrary } from "@state/library.svelte";
  import { setShuffle } from "@state/player.svelte";
  import {
    fmtTotalDuration,
    fmtTime,
    totalSeconds,
    matchesQuery,
    toPascalCase,
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
  import {
    addToPlaylist,
    getCurrentPlaylist,
    isPlaylistLoading,
    markCoverFailed,
    moveInPlaylist,
    removeFromPlaylist,
    resolvePlaylistCovers,
    setPlaylistCover,
    tracksOf,
  } from "@state/playlists.svelte";
  import { t } from "@lib/i18n";
  import AddTracksOverlay from "./AddTracksOverlay.svelte";

  let q = $state("");
  let ctxMenu = $state<{ x: number; y: number; idx: number } | null>(null);
  let removeTarget = $state<(typeof plHits)[number] | null>(null);
  let removeBusy = $state(false);

  const pl = $derived(getCurrentPlaylist());
  const library = $derived(getLibrary());
  const loading = $derived(isPlaylistLoading());

  $effect(() => {
    void pl?.id;
    q = "";
  });

  const currentTracks = $derived(pl ? tracksOf(pl) : []);
  const covers = $derived(resolvePlaylistCovers(pl?.cover, currentTracks));
  const plHits = $derived(
    q
      ? currentTracks.filter((t) =>
          matchesQuery(
            q.toLowerCase(),
            `${t.title} ${t.artist} ${t.album}`.toLowerCase(),
          ),
        )
      : currentTracks,
  );

  const { drag, start, shouldSkipClick } = useDragList({
    pathAt: (_section, idx) => plHits[idx]?.path,
    onReorder: (fromPath, toPath) => void moveTrack(fromPath, toPath),
  });

  let adding = $state(false);
  let addQuery = $state("");
  const addedPaths = $derived(new Set(pl?.paths ?? []));
  let coverInput: HTMLInputElement | undefined = $state();
  let coverBusy = $state(false);

  function totalDur(): number {
    return totalSeconds(currentTracks);
  }

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
    if (!pl) return;
    await addToPlaylist(pl.id, paths);
  }

  async function moveTrack(fromPath: string, toPath: string): Promise<void> {
    if (!pl) return;
    const from = pl.paths.indexOf(fromPath);
    const to = pl.paths.indexOf(toPath);
    if (from === to || from < 0 || to < 0) return;
    await moveInPlaylist(pl.id, from, to);
  }

  async function confirmRemoveTrack(): Promise<void> {
    const target = removeTarget;
    if (!target || !pl || removeBusy) return;
    removeBusy = true;
    const playlistName = toPascalCase(pl.name);
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
    ctxMenu = { x: e.clientX, y: e.clientY, idx };
  }

  function trackCtxItems(): CtxEntry[] {
    if (!ctxMenu || !pl) return [];
    const trk = plHits[ctxMenu.idx];
    if (!trk) return [];
    return [
      {
        label: t("playlist.addToQueue"),
        icon: ListPlus,
        action: () => addToQueue(trk.path),
      },
      { type: "separator" },
      {
        label: t("playlist.remove"),
        icon: X,
        danger: true,
        action: () => {
          removeTarget = trk;
        },
      },
    ];
  }

  function onRowPointerDown(e: PointerEvent, idx: number): void {
    if (!!q) return;
    const track = plHits[idx];
    if (track) {
      start(e, { idx, path: track.path });
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
</script>

<div class="pl-detail">
  {#if loading && !pl}
    <div class="pl-loading" role="status">
      <Loader size={18} stroke-width={1.5} class="spin" />
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
          class:busy={coverBusy}
          onclick={pickCover}
          disabled={coverBusy}
          aria-label={t("playlist.changeCover")}
        >
          <Image size={14} stroke-width={1.5} />
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
        <div class="hero-name display" aria-label={t("playlist.playlistName")}>
          {toPascalCase(pl.name)}
        </div>
        <div class="hero-sub mono">
          {#if currentTracks.length > 0}
            {currentTracks.length}
            {currentTracks.length === 1
              ? t("library.song")
              : t("library.songsPlural")} · {t("playlist.about")}
            {fmtTotalDuration(totalDur())}
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
          disabled={currentTracks.length === 0}
          aria-label={t("player.play")}
        >
          <Play size={16} fill="currentColor" stroke-width={0} />
        </button>
        <button
          class="icon-btn round"
          onclick={() => playAll(true)}
          disabled={currentTracks.length === 0}
          aria-label={t("player.shuffle")}
        >
          <Shuffle size={14} stroke-width={1.5} />
        </button>
        <button
          class="icon-btn round"
          onclick={() => (adding = true)}
          aria-label={t("playlist.addTracks")}
        >
          <Plus size={14} stroke-width={1.5} />
        </button>
      </div>
    </div>

    <div class="tracks" bind:this={drag.rowsEl}>
      <div class="tbl-head">
        <span class="h-num mono">#</span>
        <span class="h-cover"></span>
        <span class="h-title mono">{t("library.titleHeader")}</span>
        <span class="h-artist mono">{t("library.artistHeader")}</span>
        <span class="h-dur"><Clock size={12} stroke-width={2} /></span>
      </div>

      {#if plHits.length === 0}
        <div class="tbl-empty">
          <ListMusic size={30} stroke-width={1.1} />
          <span class="display"
            >{pl.paths.length === 0
              ? t("playlist.empty")
              : t("library.noMatches")}</span
          >
          <span class="mono sub"
            >{pl.paths.length === 0
              ? t("playlist.addSomeTracks")
              : t("playlist.noMatchesQuery", { query: q })}</span
          >
        </div>
      {:else}
        {#each plHits as trk, i (trk.id)}
          <div
            class="row"
            class:playing={trk.id === getCurrentTrack()?.id}
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
              {#if trk.id === getCurrentTrack()?.id}
                <EqualizerBars />
              {:else}
                {String(i + 1).padStart(2, "0")}
              {/if}
            </span>
            <div class="c-cover">
              <TrackCover src={trk.cover} shadow="var(--shadow-sm)" />
            </div>
            <div class="c-title-col">
              <span
                class="c-title"
                class:accent={trk.id === getCurrentTrack()?.id}
              >
                {trk.title}
              </span>
              {#if trk.album}
                <span class="c-album">{trk.album}</span>
              {/if}
            </div>
            <span class="c-artist">{trk.artist}</span>
            <span class="c-dur mono">{fmtTime(trk.duration, "--:--")}</span>
          </div>
        {/each}
      {/if}
    </div>

    {#if adding}
      <AddTracksOverlay
        {library}
        {addQuery}
        onQuery={(v) => (addQuery = v)}
        onClose={() => (adding = false)}
        onAddPaths={(paths) => void addPaths(paths)}
        added={addedPaths}
      />
    {/if}
  {/if}

  {#if ctxMenu}
    <ContextMenu
      x={ctxMenu.x}
      y={ctxMenu.y}
      onclose={() => (ctxMenu = null)}
      items={trackCtxItems()}
    />
  {/if}

  {#if removeTarget}
    <Dialog
      open
      title={t("playlist.removeTrackTitle")}
      description={t("playlist.removeTrackBody", {
        title: removeTarget.title,
      })}
      confirmLabel={t("playlist.removeTrackConfirm")}
      danger
      onconfirm={() => void confirmRemoveTrack()}
      oncancel={() => (removeTarget = null)}
    />
  {/if}
</div>

<style>
  .pl-detail {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
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
    background: rgba(0, 0, 0, 0.5);
    border: none;
    border-radius: 0;
    color: #fff;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .cover-pick-btn:disabled {
    cursor: default;
  }

  .cover-pick-btn.busy {
    opacity: 1;
  }

  .hero-cover-wrap:hover .cover-pick-btn:not(:disabled),
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
