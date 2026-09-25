<script lang="ts">
  import {
    getCurrentTrack,
    getUserQueue,
    getContextUpcoming,
  } from "@state/now-playing.svelte";
  import {
    playTrack,
    removeQueueTrack,
    reorderQueueTrack,
  } from "@lib/player.svelte";
  import { getCurrentShuffle } from "@state/player.svelte";
  import { blobSrc, fmtTime } from "../../lib/format";
  import { useDragList } from "@lib/drag-list.svelte";
  import { t } from "@lib/i18n";
  import type { LibraryTrack } from "@lib/types";
  import ContextMenu from "../../components/ContextMenu.svelte";
  import DragGhostLayer from "../../components/DragGhostLayer.svelte";
  import { ChevronLeft, ChevronRight, Trash2 } from "lucide-svelte";

  type QueueSection = "user" | "context";

  let collapsed = $state(false);

  let queueEl: HTMLElement | undefined = $state();
  let ctx = $state<{ x: number; y: number; path: string } | null>(null);

  const np = $derived(getCurrentTrack());
  const userQueue = $derived(getUserQueue());
  const contextUpcoming = $derived(getContextUpcoming(getCurrentShuffle()));

  function tracksOf(section: string | null) {
    return section === "context" ? contextUpcoming : userQueue;
  }

  const { drag, start, shouldSkipClick, setupWindowListeners } = useDragList({
    pathAt: (section, idx) => tracksOf(section)[idx]?.path,
    onReorder: reorderQueueTrack,
  });

  function onRowPointerDown(
    e: PointerEvent,
    section: QueueSection,
    idx: number,
  ): void {
    const track = tracksOf(section)[idx];
    if (track) {
      start(e, { section, idx, path: track.path, label: track.title });
    }
  }

  $effect(() => setupWindowListeners());

  $effect(() => {
    if (!np?.id || !queueEl) return;
    queueEl.querySelector(".now-playing")?.scrollIntoView({ block: "nearest" });
  });

</script>

<aside class="queue" class:collapsed bind:this={queueEl}>
  <div class="expanded-content">
    {#snippet qrow(q: LibraryTrack, section: QueueSection, i: number)}
      <div
        class="row"
        class:user-row={section === "user"}
        class:context-row={section === "context"}
        class:drag-over={drag.dragging &&
          drag.overIdx === i &&
          drag.grabIdx !== i}
        class:dragging={drag.dragging && drag.grabIdx === i}
        role="button"
        tabindex="-1"
        style:cursor={drag.dragging ? "grabbing" : "grab"}
        onpointerdown={(e) => onRowPointerDown(e, section, i)}
        onclick={() => {
          if (shouldSkipClick()) return;
          playTrack(q.id);
        }}
        onkeydown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            playTrack(q.id);
          }
        }}
        oncontextmenu={(e) => {
          e.preventDefault();
          ctx = { x: e.clientX, y: e.clientY, path: q.path };
        }}
      >
        <div class="q-cover">
          {#if q.cover}
            <img
              class="q-cover-img"
              use:blobSrc={q.cover}
              alt=""
              loading="lazy"
              draggable="false"
            />
          {:else}
            <div class="q-cover-ph"></div>
          {/if}
        </div>
        <div class="q-meta">
          <span class="q-title">{q.title}</span>
          {#if "artist" in q && q.artist}
            <span class="q-artist">{q.artist}</span>
          {/if}
        </div>
        <span class="q-dur mono">{fmtTime(q.duration, "--:--")}</span>
      </div>
    {/snippet}

    <div class="queue-head">
      <div class="queue-head-left">
        <button
          class="collapse-btn"
          aria-label={t("player.collapseQueue")}
          onclick={() => (collapsed = true)}
        >
          <ChevronRight size={13} stroke-width={1.5} />
        </button>
        <span class="mono">{t("player.queue")}</span>
      </div>
      {#if np}
        <span class="queue-count mono">{userQueue.length + contextUpcoming.length + 1}</span>
      {/if}
    </div>

    {#if np}
        <div class="now-playing">
          <div class="q-cover">
            {#if np.cover}
              <img
                class="q-cover-img"
                use:blobSrc={np.cover}
                alt=""
                loading="lazy"
                draggable="false"
              />
            {:else}
              <div class="q-cover-ph"></div>
            {/if}
          </div>
          <div class="q-meta">
            <span class="q-title accent">{np.title}</span>
            {#if "artist" in np && np.artist}
              <span class="q-artist">{np.artist}</span>
            {/if}
          </div>
          <span class="q-dur mono">{fmtTime(np.duration, "--:--")}</span>
        </div>

      <div class="queue-body" bind:this={drag.rowsEl}>
        {#if userQueue.length > 0}
          <span class="section-label mono">QUEUE</span>
          {#each userQueue as q, i (q.path)}
            {@render qrow(q, "user", i)}
          {/each}
        {/if}

        {#if contextUpcoming.length > 0}
          <span class="section-label mono">NEXT UP</span>
          {#each contextUpcoming as q, i (q.path)}
            {@render qrow(q, "context", i)}
          {/each}
        {/if}

        {#if userQueue.length === 0 && contextUpcoming.length === 0}
          <div class="queue-empty">
            <span class="mono">{t("player.nothingPlaying")}</span>
            <span class="queue-empty-sub mono">{t("player.pickFromLibrary")}</span>
          </div>
        {/if}
      </div>

    {:else}
      <div class="queue-empty">
        <span class="mono">{t("player.nothingPlaying")}</span>
        <span class="queue-empty-sub mono">{t("player.pickFromLibrary")}</span>
      </div>
    {/if}

    {#if ctx}
      <ContextMenu
        x={ctx.x}
        y={ctx.y}
        onclose={() => (ctx = null)}
        items={[
          {
            label: t("player.removeFromQueue"),
            icon: Trash2,
            danger: true,
            action: () => removeQueueTrack(ctx!.path),
          },
        ]}
      />
    {/if}
  </div>

  <div class="collapsed-view">
    <button
      class="expand-btn"
      aria-label={t("player.expandQueue")}
      onclick={() => (collapsed = false)}
    >
      <ChevronLeft size={18} stroke-width={1.5} />
    </button>
  </div>
</aside>

<DragGhostLayer ghost={drag.ghost} />

<style>
  .queue {
    grid-column: 2;
    grid-row: 1;
    width: 320px;
    min-height: 0;
    min-width: 0;
    padding: 18px 14px;
    position: relative;
    z-index: 2;
    background: var(--bg);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    transition:
      width 0.28s ease-in-out,
      padding 0.28s ease-in-out;
  }

  .expanded-content {
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    opacity: 1;
    transition: opacity 0.22s ease;
  }

  .queue.collapsed .expanded-content {
    opacity: 0;
    pointer-events: none;
  }

  .queue.collapsed {
    width: 28px;
    padding: 0;
    overflow: hidden;
  }

  .collapsed-view {
    position: absolute;
    inset: 0;
    z-index: 3;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.22s ease;
  }

  .queue.collapsed .collapsed-view {
    opacity: 1;
    pointer-events: auto;
  }

  @container player (max-width: 560px) {
    .queue {
      display: none;
    }
  }

  .expand-btn {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    color: var(--text);
    background: none;
    border: none;
    z-index: 3;
    opacity: 1;
    transition:
      color 0.15s,
      transform 0.15s,
      opacity 0.18s ease-out,
      background 0.15s;
  }

  .expand-btn:hover {
    color: var(--accent);
    transform: translate(-50%, -50%) scale(1.15);
  }

  .expand-btn:active {
    transform: translate(-50%, -50%) scale(0.95);
  }


  .queue-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 0 4px 10px;
    font-size: 10px;
    color: var(--faint);
    letter-spacing: 0.14em;
    text-transform: uppercase;
    border-bottom: 1px solid var(--line);
  }

  .queue-head-left {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .collapse-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 3px;
    color: var(--faint);
    transition:
      color 0.1s,
      background 0.1s;
  }

  .collapse-btn:hover {
    color: var(--text);
    background: rgba(255, 255, 255, 0.06);
  }

  .queue-count {
    position: relative;
    z-index: 0;
    width: 24px;
    height: 24px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 9.5px;
    color: var(--dim);
    line-height: 1;
    letter-spacing: 0.04em;
  }

  .queue-count::before {
    content: "";
    position: absolute;
    inset: 0;
    z-index: -1;
    transform: rotate(45deg);
    background: var(--bg-raise);
    border: 1px solid var(--line-strong);
    border-radius: 5px;
  }

  .now-playing {
    display: grid;
    grid-template-columns: 36px 1fr 42px;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
  }

  .section-label {
    display: block;
    padding: 4px 10px;
    font-size: 9px;
    color: var(--faint);
    letter-spacing: 0.14em;
  }

  .queue-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: none;
    padding: 6px 2px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .queue-body::-webkit-scrollbar {
    display: none;
  }

  .row {
    position: relative;
    display: grid;
    grid-template-columns: 36px 1fr 42px;
    align-items: center;
    gap: 10px;
    width: 100%;
    text-align: left;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    font-size: 11.5px;
    color: var(--dim);
    transition: background 0.12s;
    border: none;
    outline: none;
    user-select: none;
    cursor: pointer;
  }

  .row:hover {
    background: var(--bg-raise);
  }

  .context-row {
    cursor: pointer;
  }

  .row.dragging {
    opacity: 0.3;
  }

  .q-cover {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg-raise);
    flex-shrink: 0;
  }

  .q-cover-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .q-cover-ph {
    width: 100%;
    height: 100%;
    background: var(--placeholder-gradient);
    border: 1px solid var(--line);
  }

  .q-meta {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .q-title {
    font-size: 12px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.2;
  }

  .q-title.accent {
    color: var(--accent);
  }

  .q-artist {
    font-size: 10.5px;
    color: var(--dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .q-dur {
    font-size: 11px;
    color: var(--faint);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .queue-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    text-align: center;
    color: var(--faint);
    font-size: 12px;
  }

  .queue-empty-sub {
    font-size: 10px;
    color: var(--faint);
    opacity: 0.6;
  }
</style>