<script lang="ts">
  import {
    getCurrentTrack,
    getUserQueue,
    getContextUpcoming,
    getPlayToken,
  } from "@state/now-playing.svelte";
  import {
    playTrack,
    removeQueueTrack,
    reorderQueueTrack,
  } from "@lib/playback.svelte";
  import { getCurrentLoop, getCurrentShuffle } from "@state/player.svelte";
  import { fmtTime } from "../../lib/format";
  import { useDragList } from "@lib/drag-list.svelte";
  import { t } from "@lib/i18n";
  import { fade, slide } from "svelte/transition";
  import type { LibraryTrack } from "@lib/types";
  import ContextMenu from "../../components/ContextMenu.svelte";
  import VirtualList from "../../components/VirtualList.svelte";
  import TrackCover from "../../components/TrackCover.svelte";
  import { ChevronRight, Trash2 } from "lucide-svelte";
  import { setQueueOpen } from "@state/queue.svelte";

  type QueueSection = "user" | "context";

  let queueBodyEl: HTMLDivElement | undefined = $state();
  let ctx = $state<{ x: number; y: number; path: string } | null>(null);

  const calm =
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  const np = $derived(getCurrentTrack());
  const playToken = $derived(getPlayToken());
  const repeating = $derived(getCurrentLoop() === "one");
  const userQueue = $derived(getUserQueue());
  const contextUpcoming = $derived(
    getContextUpcoming(getCurrentShuffle(), getCurrentLoop()),
  );
  const upcomingCount = $derived(
    repeating ? 1 : userQueue.length + contextUpcoming.length + 1,
  );
  const nextUpLabel = $derived.by(() =>
    getCurrentLoop() === "all"
      ? t("player.nextUpRepeatAll")
      : t("player.nextUp"),
  );

  function tracksOf(section: string | null) {
    return section === "context" ? contextUpcoming : userQueue;
  }

  const { drag, start, shouldSkipClick } = useDragList({
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
      start(e, {
        section,
        idx,
        path: track.path,
        paths: tracksOf(section).map((t) => t.path),
      });
    }
  }

  $effect(() => {
    if (!np?.id || !queueBodyEl) return;
    void playToken;
    queueBodyEl.scrollTo({
      top: 0,
      behavior: calm ? "auto" : "smooth",
    });
  });
</script>

<aside class="queue" transition:slide={{ axis: "x", duration: 240 }}>
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
        tabindex="0"
        aria-label={`${q.title} — ${"artist" in q && q.artist ? q.artist : ""}`}
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
          <TrackCover src={q.cover} />
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
          aria-label={t("player.closeQueue")}
          onclick={() => setQueueOpen(false)}
        >
          <ChevronRight size={13} stroke-width={1.5} />
        </button>
        <span class="mono">{t("common.queue")}</span>
      </div>
      {#if np}
        <span class="queue-count mono">{upcomingCount}</span>
      {/if}
    </div>

    {#if np}
      <div class="np-slot">
        {#key `${np.id}:${playToken}`}
          <div
            class="now-playing"
            in:fade={{ duration: calm ? 0 : 200 }}
            out:fade={{ duration: calm ? 0 : 140 }}
          >
            <div class="q-cover">
              <TrackCover src={np.cover} />
            </div>
            <div class="q-meta">
              <span class="q-title accent">{np.title}</span>
              {#if "artist" in np && np.artist}
                <span class="q-artist">{np.artist}</span>
              {/if}
            </div>
            <span class="q-dur mono">{fmtTime(np.duration, "--:--")}</span>
          </div>
        {/key}
      </div>

      <div class="queue-body" bind:this={drag.rowsEl} bind:this={queueBodyEl}>
        {#if !repeating && userQueue.length > 0}
          <div class="queue-section" in:fade={{ duration: calm ? 0 : 180 }}>
            <span class="section-label mono">{t("common.queue")}</span>
            {#snippet userRow(q: LibraryTrack, i: number)}
              {@render qrow(q, "user", i)}
            {/snippet}
            <VirtualList
              items={userQueue}
              row={userRow}
              scrollEl={queueBodyEl ?? null}
              rowKey={(q) => q.path}
              estimateRowHeight={48}
              rowGap={1}
              smooth
            />
          </div>
        {/if}

        {#if contextUpcoming.length > 0}
          <div class="queue-section" in:fade={{ duration: calm ? 0 : 180 }}>
            <span class="section-label mono">{nextUpLabel}</span>
            {#snippet contextRow(q: LibraryTrack, i: number)}
              {@render qrow(q, "context", i)}
            {/snippet}
            <VirtualList
              items={contextUpcoming}
              row={contextRow}
              scrollEl={queueBodyEl ?? null}
              rowKey={(q) => q.path}
              estimateRowHeight={48}
              rowGap={1}
              smooth
            />
          </div>
        {/if}

        {#if userQueue.length === 0 && contextUpcoming.length === 0}
          <div class="queue-empty">
            {#if repeating}
              <span class="mono">{t("player.nothingQueued")}</span>
              <span class="queue-empty-sub mono">{t("player.repeating")}</span>
            {:else}
              <span class="mono">{t("player.nothingPlaying")}</span>
              <span class="queue-empty-sub mono">
                {t("player.pickFromLibrary")}
              </span>
            {/if}
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
        onClose={() => (ctx = null)}
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
</aside>

<style>
  .queue {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 5;
    width: 320px;
    padding: 18px 14px;
    min-height: 0;
    background: var(--surface);
    border-left: 1px solid var(--line);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .expanded-content {
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  @container player (max-width: 560px) {
    .queue {
      display: none;
    }
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
    border-radius: var(--radius-sm);
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
    border-radius: var(--radius-sm);
  }

  .np-slot {
    position: relative;
    flex: none;
    height: 48px;
  }

  .now-playing {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-columns: 36px 1fr 42px;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
  }

  .queue-section {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .section-label {
    display: block;
    padding: 4px 10px;
    font-size: 9px;
    color: var(--faint);
    letter-spacing: 0.14em;
    text-transform: uppercase;
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
    display: flex;
    --cover-size: 36px;
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
