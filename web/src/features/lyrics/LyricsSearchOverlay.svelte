<script lang="ts">
  import { untrack } from "svelte";
  import { t } from "@lib/i18n";
  import { vynl } from "@lib/vynl";
  import type { LrcSearchResult, LyricsResult } from "@lib/types";
  import { getCurrentId, getCurrentTrack } from "@state/now-playing.svelte";
  import { lyricsKindOf, setLyricsManual, toLyricLines } from "@state/lyrics.svelte";

  let { onClose }: { onClose: () => void } = $props();

  let query = $state("");
  let results = $state<LrcSearchResult[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let version = 0;
  let autoQuery = "";

  $effect(() => {
    const id = getCurrentId();
    untrack(() => {
      const track = getCurrentTrack();
      const next = track ? `${track.title} ${track.artist}`.trim() : "";
      autoQuery = next;
      query = next;
      results = [];
      error = null;
      if (id && next) void run(next, true);
    });
  });

  async function run(q = query, scoped = q.trim() === autoQuery.trim()): Promise<void> {
    const trimmed = q.trim();
    if (!trimmed) return;
    const ver = ++version;
    loading = true;
    error = null;
    try {
      const track = getCurrentTrack();
      const data = await vynl.lyricsSearch({
        query: trimmed,
        ...(scoped && track ? { trackName: track.title, artistName: track.artist } : {}),
      });
      if (ver === version) results = data;
    } catch (e) {
      if (ver === version) {
        results = [];
        error = e instanceof Error && e.message ? e.message : t("lyrics.searchFailed");
      }
    } finally {
      if (ver === version) loading = false;
    }
  }

  function apply(item: LrcSearchResult): void {
    const text = item.syncedLyrics ?? item.plainLyrics ?? "";
    if (!text) return;
    const result: LyricsResult = { kind: lyricsKindOf(text), text, source: "remote" };
    setLyricsManual(result, toLyricLines(result));
    onClose();
  }
</script>

<div class="row">
  <input
    class="input mono"
    type="text"
    bind:value={query}
    placeholder={t("lyrics.searchPlaceholder")}
    onkeydown={(e) => e.key === "Enter" && run()}
  />
  <button class="btn" onclick={() => run()} disabled={loading}>{t("lyrics.search")}</button>
</div>

{#if results.length > 0}
  <div class="results">
    {#each results as item (item.id)}
      <button class="item" onclick={() => apply(item)}>
        <div class="track display">{item.trackName}</div>
        <div class="artist mono">{item.artistName}{item.albumName ? ` · ${item.albumName}` : ""}</div>
        <div class="meta mono">
          {item.syncedLyrics ? t("lyrics.synced") : t("lyrics.plain")}{#if item.duration} · {Math.round(item.duration)}s{/if}
        </div>
      </button>
    {/each}
  </div>
{:else if !loading && query.trim()}
  <div class="empty mono" class:bad={!!error}>{error ?? t("lyrics.noResults")}</div>
{/if}

<style>
  .row {
    display: flex;
    gap: 8px;
  }

  .input {
    flex: 1;
    min-width: 0;
    padding: 7px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line-strong);
    background: var(--bg-raise);
    color: var(--text);
    font-size: 11px;
    outline: none;
  }

  .results {
    max-height: 300px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    scrollbar-width: none;
  }

  .results::-webkit-scrollbar {
    display: none;
  }

  .item {
    text-align: left;
    padding: 8px 10px;
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: inherit;
    cursor: pointer;
  }

  .item:hover {
    background: var(--bg-raise);
  }

  .track {
    font-size: 13px;
  }

  .artist,
  .meta {
    font-size: 10px;
    color: var(--faint);
    letter-spacing: 0.05em;
  }

  .meta {
    color: var(--dim);
  }

  .empty {
    padding: 20px 0;
    text-align: center;
    font-size: 11px;
    color: var(--faint);
  }

  .empty.bad {
    color: var(--red);
  }

  .btn {
    appearance: none;
    border: 1px solid color-mix(in srgb, var(--accent) 58%, transparent);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--dim);
    padding: 7px 16px;
    font-size: 10.5px;
    letter-spacing: 0.06em;
    cursor: pointer;
  }

  .btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
</style>