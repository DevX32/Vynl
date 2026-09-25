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
  import { vynl } from "../../lib/vynl";
  import SearchInput from "../../components/SearchInput.svelte";
  import ContextMenu, {
    type CtxEntry,
  } from "../../components/ContextMenu.svelte";
  import DragGhostLayer from "../../components/DragGhostLayer.svelte";
  import {
    addToPlaylist,
    getCurrentPlaylist,
    moveInPlaylist,
    removeFromPlaylist,
    setPlaylistCover,
  } from "@state/playlists.svelte";
  import { t } from "@lib/i18n";
  import AddTracksOverlay from "./AddTracksOverlay.svelte";

  let q = $state("");
  let ctxMenu = $state<{ x: number; y: number; idx: number } | null>(null);

  const pl = $derived(getCurrentPlaylist());
  const library = $derived(getLibrary());

  $effect(() => {
    void pl?.id;
    q = "";
  });

  const currentTracks = $derived(
    pl
      ? pl.paths
          .map((p) => library.find((t) => t.path === p))
          .filter((t): t is import("../../lib/types").LibraryTrack => !!t)
      : [],
  );
  const covers = $derived(
    pl?.cover
      ? [pl.cover]
      : [
          ...new Set(
            currentTracks.map((t) => t.cover).filter((c): c is string => !!c),
          ),
        ].slice(0, 4),
  );
  const plHits = $derived(
    q
      ? currentTracks.filter((t) =>
          matchesQuery(q.toLowerCase(), `${t.title} ${t.artist} ${t.album}`.toLowerCase()),
        )
      : currentTracks,
  );

  const { drag, start, shouldSkipClick, setupWindowListeners } = useDragList({
    pathAt: (_section, idx) => plHits[idx]?.path,
    onReorder: (fromPath, toPath) => void moveTrack(fromPath, toPath),
  });

  let adding = $state(false);
  let addQuery = $state("");
  const addedPaths = $derived(new Set(pl?.paths ?? []));
  let coverInput: HTMLInputElement | undefined = $state();

  function totalDur(): number {
    return totalSeconds(currentTracks);
  }

  function playAll(shuffle = false): void {
    if (currentTracks.length === 0 || !pl) return;
    if (shuffle) setShuffle(true);
    const first = shuffle
      ? currentTracks[Math.floor(Math.random() * currentTracks.length)]
      : currentTracks[0];
    requestPlay(first.id, currentTracks.map((track) => track.path));
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

  async function dropTrack(trackPath: string): Promise<void> {
    if (!pl) return;
    await removeFromPlaylist(pl.id, trackPath);
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
        action: () => void dropTrack(trk.path),
      },
    ];
  }

  function onRowPointerDown(e: PointerEvent, idx: number): void {
    if (!!q) return;
    const track = plHits[idx];
    if (track) {
      start(e, { idx, path: track.path, label: track.title });
    }
  }

  $effect(() => setupWindowListeners());

  function pickCover(): void {
    coverInput?.click();
  }

  async function onCoverFile(e: Event): Promise<void> {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file || !pl) return;
    const ext = file.name.split(".").pop() || "jpg";
    const arrayBuf = await file.arrayBuffer();
    const data = Array.from(new Uint8Array(arrayBuf));
    const savedPath = await vynl.savePlaylistCover(pl.id, ext, data);
    await setPlaylistCover(pl.id, savedPath);
    input.value = "";
  }
</script>

<div class="pl-detail">
  {#if pl}
    <div class="hero">
      <div class="hero-cover-wrap">
        {#if covers[0]}
          <img
            class="hero-cover"
            use:blobSrc={covers[0]}
            alt=""
            loading="lazy"
            draggable="false"
          />
        {:else}
          <div class="hero-cover placeholder">
            <Disc3 size={22} stroke-width={1} />
          </div>
        {/if}
        <button
          class="cover-pick-btn"
          onclick={pickCover}
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
              : t("playlist.noMatchesQuery", { q })}</span
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
            tabindex="-1"
            style:cursor={!q ? (drag.dragging ? "grabbing" : "grab") : "pointer"}
            onpointerdown={(e) => onRowPointerDown(e, i)}
            oncontextmenu={(e) => showTrackCtx(e, i)}
            onclick={() => {
              if (shouldSkipClick()) return;
              requestPlay(trk.id, pl.paths);
            }}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); requestPlay(trk.id, pl.paths); } }}
          >
            <span class="c-num mono">
              {#if trk.id === getCurrentTrack()?.id}
                <span class="eq">
                  <span class="eq-bar"></span>
                  <span class="eq-bar"></span>
                  <span class="eq-bar"></span>
                </span>
              {:else}
                {String(i + 1).padStart(2, "0")}
              {/if}
            </span>
            <div class="c-cover">
              {#if trk.cover}
                <img
                  class="cover-img"
                  use:blobSrc={trk.cover}
                  alt=""
                  loading="lazy"
                  draggable="false"
                />
              {:else}
                <div class="cover-placeholder"></div>
              {/if}
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
</div>

<DragGhostLayer ghost={drag.ghost} />

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

  .hero-cover {
    width: 96px;
    height: 96px;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
    object-fit: cover;
    display: block;
    background: var(--bg-raise);
    box-shadow: var(--shadow-sm);
  }

  .hero-cover.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--faint);
    border: 1px solid var(--line);
    background: var(--placeholder-gradient);
  }

  .hero-cover-wrap {
    position: relative;
    flex-shrink: 0;
  }

  .cover-pick-btn {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.5);
    border: none;
    border-radius: var(--radius-sm);
    color: #fff;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .hero-cover-wrap:hover .cover-pick-btn {
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
    --track-cols: 28px 40px minmax(0, 1fr) minmax(0, 1fr) 42px;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: none;
    display: flex;
    flex-direction: column;
    padding-right: 2px;
    padding-bottom: 8px;
  }

  .tracks::-webkit-scrollbar {
    display: none;
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
    width: 40px;
    height: 40px;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg-raise);
    box-shadow: var(--shadow-sm);
  }

  .cover-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .cover-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--faint);
    border: 1px solid var(--line);
    background: var(--placeholder-gradient);
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

  .eq {
    display: inline-flex;
    align-items: flex-end;
    gap: 2px;
    height: 14px;
  }

  .eq-bar {
    width: 3px;
    border-radius: 1px;
    background: var(--accent);
    animation: eq 0.8s ease-in-out infinite alternate;
  }

  .eq-bar:nth-child(1) { height: 40%; animation-delay: 0s; }
  .eq-bar:nth-child(2) { height: 70%; animation-delay: 0.2s; }
  .eq-bar:nth-child(3) { height: 50%; animation-delay: 0.4s; }

  @keyframes eq {
    0% { height: 20%; }
    100% { height: 100%; }
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

</style>