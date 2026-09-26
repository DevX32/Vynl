<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import {
    Play,
    Pause,
    Shuffle,
    ListMusic,
    ListPlus,
    Trash2,
    Clock,
    ListMusic as ListMusicIcon,
  } from "lucide-svelte";
  import { vynl } from "../../lib/vynl";
  import { refreshLibrary, getLibrary } from "@state/library.svelte";
  import {
    getCurrentTrack,
    getIsTrackPlaying,
    requestPlay,
    addToQueue,
    addToQueueFront,
  } from "@state/now-playing.svelte";
  import { toggle } from "@lib/player.svelte";
  import { setShuffle } from "@state/player.svelte";
  import { isInputField } from "@state/shortcuts.svelte";
  import {
    fmtTotalDuration,
    fmtTime,
    totalSeconds,
    matchesQuery,
    toPascalCase,
  } from "../../lib/format";
  import { t } from "@lib/i18n";
  import { toasts } from "@lib/toast";
  import { blobSrc } from "../../lib/format";
  import SearchInput from "../../components/SearchInput.svelte";
  import Dialog from "../../components/Dialog.svelte";
  import ContextMenu, { type CtxEntry } from "../../components/ContextMenu.svelte";
  import {
    addToPlaylist,
    getCurrentPlaylists,
    refreshPlaylists,
  } from "@state/playlists.svelte";

  let {
    active,
  }: {
    active: boolean;
  } = $props();

  let scanning = $state(false);
  let query = $state("");
  let justAdded = $state(new Set<string>());
  let searchInput: SearchInput | undefined = $state();

  let ctx = $state<{
    x: number;
    y: number;
    track: (typeof hits)[number];
  } | null>(null);

  let deleteTarget = $state<(typeof hits)[number] | null>(null);

  const library = $derived(getLibrary());
  const nowPlaying = $derived(getCurrentTrack());
  const playing = $derived(getIsTrackPlaying());
  const playlists = $derived(getCurrentPlaylists());

  const titleCollator = new Intl.Collator();
  const sorted = $derived(
    [...library].sort((a, b) => titleCollator.compare(a.title, b.title)),
  );
  const hits = $derived(
    query
      ? sorted.filter((t) =>
          matchesQuery(
            query.toLowerCase(),
            `${t.title} ${t.artist} ${t.album}`.toLowerCase(),
          ),
        )
      : sorted,
  );

  $effect(() => {
    if (active) void refresh();
  });

  async function refresh(): Promise<void> {
    scanning = true;
    try {
      await Promise.all([refreshLibrary(), refreshPlaylists()]);
    } catch (e) {
      console.warn("refresh failed:", e);
    } finally {
      scanning = false;
    }
  }

  function ctxMenuItems(): CtxEntry[] {
    if (!ctx) return [];
    const trk = ctx.track;
    const isPlaying = trk.id === nowPlaying?.id && playing;
    return [
      {
        label: isPlaying ? t("contextMenu.pause") : t("contextMenu.play"),
        icon: isPlaying ? Pause : Play,
        action: () => rowAction(trk),
      },
      {
        label: t("player.playNext"),
        icon: ListPlus,
        action: () => addToQueueFront(trk.path),
      },
      {
        label: t("player.addToQueue"),
        icon: ListPlus,
        action: () => addToQueue(trk.path),
      },
      { type: "separator" },
      {
        label: t("library.addToPlaylist"),
        items: playlists.map((p) => ({
          label: toPascalCase(p.name),
          icon: ListMusicIcon,
          action: () => void addTo(p.id, trk.path),
          secondary: justAdded.has(`${p.id}:${trk.path}`) ? t("trackRow.done") : undefined,
        })),
      },
      { type: "separator" },
      {
        label: t("contextMenu.delete"),
        icon: Trash2,
        danger: true,
        action: () => {
          deleteTarget = trk;
        },
      },
    ];
  }

  onMount(() => {
    const onKey = (e: KeyboardEvent): void => {
      if (!active || isInputField(e.target)) return;
      if (e.key === "/") {
        e.preventDefault();
        searchInput?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  function totalDur(): number {
    return totalSeconds(hits);
  }

  function playAll(shuffle = false): void {
    if (hits.length === 0) return;
    if (shuffle) setShuffle(true);
    const first = shuffle
      ? hits[Math.floor(Math.random() * hits.length)]
      : hits[0];
    requestPlay(first.id, hits.map((track) => track.path));
  }

  function rowAction(t: (typeof hits)[number]): void {
    if (nowPlaying?.id === t.id) {
      toggle();
    } else {
      requestPlay(
        t.id,
        hits.map((x) => x.path),
      );
    }
  }

  async function addTo(plId: string, path: string): Promise<void> {
    const key = `${plId}:${path}`;
    await addToPlaylist(plId, [path]);
    justAdded = new Set([...justAdded, key]);
    window.setTimeout(() => {
      justAdded = new Set([...justAdded].filter((k) => k !== key));
      if (ctx?.track.path === path) ctx = null;
    }, 1200);
  }

  async function handleDelete(): Promise<void> {
    const target = deleteTarget;
    if (!target) return;
    try {
      await vynl.deleteLibraryTrack(target.path);
      await refreshLibrary();
      toasts.success(t("library.deleteConfirm"));
    } catch {
      toasts.error(t("library.deleteFailed"));
    }
    deleteTarget = null;
  }

  function onRowContext(e: MouseEvent, t: (typeof hits)[number]): void {
    e.preventDefault();
    ctx = { x: e.clientX, y: e.clientY, track: t };
  }
</script>

<section class="library-page">
  <div class="view" in:fade={{ duration: 150 }}>
    <div class="hdr">
      <div class="hdr-copy">
        <div class="kicker mono">{t("library.title")}</div>
        <div class="title display">{t("library.songs")}</div>
        <div class="sub mono">
          {#if hits.length > 0}
            {hits.length}
            {hits.length === 1 ? t("library.song") : t("library.songsPlural")} ·
            {fmtTotalDuration(totalDur())}
          {:else}
            {t("playlist.zeroSongs")}
          {/if}
          {#if scanning}
            · {t("player.scanning")}{/if}
        </div>
      </div>
      <div class="hdr-actions">
        <SearchInput
          bind:this={searchInput}
          bind:value={query}
          placeholder={t("search.placeholder")}
          oninput={(q) => (query = q)}
        />
        <button
          class="play-btn"
          onclick={() => playAll(false)}
          disabled={hits.length === 0}
          aria-label={t("library.playAll")}
        >
          <Play size={16} fill="currentColor" stroke-width={0} />
        </button>
        <button
          class="shuffle-btn"
          onclick={() => playAll(true)}
          disabled={hits.length === 0}
          aria-label={t("player.shuffle")}
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
        <span class="h-actions"></span>
      </div>

      {#if hits.length === 0}
        <div class="tbl-empty">
          <ListMusic size={28} stroke-width={1.1} />
          <span class="empty-title display"
            >{query ? t("library.noMatches") : t("library.nothingHere")}</span
          >
          <span class="empty-sub mono"
            >{query
              ? t("library.noMatchesQuery", { query })
              : t("library.pasteToStart")}</span
          >
        </div>
      {:else}
        {#each hits as trk, i (trk.id)}
          <button
            type="button"
            class="row"
            class:playing={trk.id === nowPlaying?.id}
            onclick={() => rowAction(trk)}
            oncontextmenu={(e) => onRowContext(e, trk)}
          >
            <span class="c-num mono">
              {#if trk.id === nowPlaying?.id}
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
                />
              {:else}
                <div class="cover-placeholder"></div>
              {/if}
            </div>
            <div class="c-meta">
              <span class="c-title" class:accent={trk.id === nowPlaying?.id}>
                {trk.title}
              </span>
              {#if trk.album}
                <span class="c-album">{trk.album}</span>
              {/if}
            </div>
            <span class="c-artist">{trk.artist}</span>
            <span class="c-dur mono">{fmtTime(trk.duration, "--:--")}</span>
            <span class="c-actions"></span>
          </button>
        {/each}
      {/if}
    </div>
  </div>
</section>

{#if ctx}
  <ContextMenu
    x={ctx.x}
    y={ctx.y}
    items={ctxMenuItems()}
    onclose={() => (ctx = null)}
  />
{/if}

{#if deleteTarget}
  <Dialog
    open
    title={t("library.deleteTitle")}
    description={t("library.deleteBody", { title: deleteTarget.title })}
    confirmLabel={t("library.deleteConfirm")}
    danger
    onconfirm={() => void handleDelete()}
    oncancel={() => (deleteTarget = null)}
  />
{/if}

<style>
  .library-page {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .view {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .hdr {
    flex-shrink: 0;
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    padding: 20px 0 12px;
  }

  .title {
    font-size: 42px;
    line-height: 1;
    letter-spacing: -0.02em;
    margin: 0;
  }

  .sub {
    font-size: 11px;
    color: var(--dim);
    margin-top: 8px;
  }

  .hdr-actions {
    display: flex;
    align-items: center;
    gap: 18px;
    margin-right: 16px;
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
  }

  .shuffle-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .tracks {
    --track-cols: 28px 40px minmax(0, 1fr) minmax(0, 1fr) 42px 28px;
    --col-gap: 12px;
    --row-px: 10px;
    --row-py: 7px;

    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding-bottom: 24px;
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
    gap: var(--col-gap);
    padding: 8px var(--row-px);
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
    grid-template-columns: var(--track-cols);
    align-items: center;
    gap: var(--col-gap);
    padding: var(--row-py) var(--row-px);
    border-radius: var(--radius-sm);
    transition: background 0.12s;
    cursor: default;
    background: none;
    border: none;
    color: inherit;
    text-align: left;
    width: 100%;
  }

  .row:hover {
    background: var(--bg-raise);
  }

  .row.playing {
    background: var(--accent-soft);
  }

  .c-num {
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

  .c-cover {
    width: 40px;
    height: 40px;
  }

  .cover-img {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-sm);
    object-fit: cover;
    background: var(--bg-raise);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.15);
  }

  .cover-placeholder {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-sm);
    background: var(--placeholder-gradient);
    border: 1px solid var(--line);
  }

  .c-meta {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 2px;
    min-width: 0;
  }

  .c-title {
    font-size: 13px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .c-title.accent {
    color: var(--accent);
  }

  .c-artist {
    font-size: 11.5px;
    color: var(--dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .c-album {
    font-size: 10.5px;
    color: var(--faint);
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

  .tbl-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--faint);
  }

  .empty-title {
    font-size: 24px;
    color: var(--dim);
  }

  .empty-sub {
    font-size: 10.5px;
    letter-spacing: 0.04em;
  }

  @container player (max-width: 620px) {
    .hdr {
      padding: 14px 0 10px;
      gap: 10px;
    }

    .title {
      font-size: 32px;
    }

    .hdr-actions {
      gap: 10px;
      margin-right: 10px;
    }

    .tracks {
      --track-cols: 24px 36px minmax(0, 1fr) minmax(0, 1fr) 36px;
      --col-gap: 8px;
    }

    .c-actions,
    .h-actions {
      display: none;
    }
  }

  @container player (max-width: 420px) {
    .hdr {
      flex-wrap: wrap;
      padding: 10px 0 8px;
      gap: 8px;
    }

    .title {
      font-size: 26px;
      width: 100%;
    }

    .hdr-actions {
      margin-right: 0;
    }

    .tracks {
      --track-cols: 20px minmax(0, 1fr) minmax(0, 1fr);
      --col-gap: 6px;
      --row-px: 8px;
      --row-py: 6px;
    }

    .c-cover,
    .h-cover,
    .c-dur,
    .h-dur,
    .c-actions,
    .h-actions {
      display: none;
    }
  }
</style>