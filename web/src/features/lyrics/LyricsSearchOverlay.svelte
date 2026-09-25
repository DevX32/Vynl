<script lang="ts">
  import { t } from "@lib/i18n";
  import { vynl } from "../../lib/vynl";
  import type { LyricsResult, LrcSearchResult } from "../../lib/types";
  import { getCurrentTrack } from "@state/now-playing.svelte";
  import {
    setLyricsManual,
    parseLrcText,
    toLyricLines,
  } from "@state/lyrics.svelte";

  let {
    onClose,
  }: {
    onClose: () => void;
  } = $props();

  let searchQuery = $state("");
  let searchResults = $state<LrcSearchResult[]>([]);
  let searchLoading = $state(false);
  let searchVersion = 0;

  $effect(() => {
    const track = getCurrentTrack();
    searchQuery = track ? `${track.title} ${track.artist}` : "";
    searchResults = [];
    if (searchQuery.trim()) void runSearch();
  });

  async function runSearch(): Promise<void> {
    const query = searchQuery.trim();
    if (!query) return;
    searchLoading = true;
    const ver = ++searchVersion;
    try {
      const track = getCurrentTrack();
      const results = await vynl.lyricsSearch({
        query,
        trackName: track?.title,
        artistName: track?.artist,
      });
      if (ver === searchVersion) searchResults = results;
    } catch {
      if (ver === searchVersion) searchResults = [];
    } finally {
      if (ver === searchVersion) searchLoading = false;
    }
  }

  function applySearchResult(item: LrcSearchResult): void {
    const text = item.syncedLyrics ?? item.plainLyrics ?? "";
    if (!text) return;

    const synced = !!item.syncedLyrics;
    const result: LyricsResult = { kind: synced ? "lrc" : "txt", text, source: "remote" };
    setLyricsManual(result, synced ? parseLrcText(text) : toLyricLines(result));
    onClose();

    const track = getCurrentTrack();
    if (!track?.path) return;
    const plain = synced
      ? text.replace(/^\[\d{1,2}:\d{2}(?:[.:]\d{1,3})?\]\s*/gm, "").trim()
      : text;
    if (plain) void vynl.lyricsEmbed({ file: track.path, text: plain }).catch(() => {});
  }
</script>

<div class="search-input-row">
  <input
    class="search-input mono"
    type="text"
    bind:value={searchQuery}
    placeholder={t("lyrics.searchPlaceholder")}
    onkeydown={(e) => e.key === "Enter" && runSearch()}
  />
  <button class="btn-primary" onclick={runSearch} disabled={searchLoading}>
    {searchLoading ? "…" : t("lyrics.search")}
  </button>
</div>

{#if searchResults.length > 0}
  <div class="search-results">
    {#each searchResults as item (item.id)}
      <button class="search-item" onclick={() => applySearchResult(item)}>
        <div class="search-item-track display">{item.trackName}</div>
        <div class="search-item-artist mono">{item.artistName} · {item.albumName}</div>
        <div class="search-item-meta mono">
          {item.syncedLyrics ? t("lyrics.synced") : t("lyrics.plain")}{#if item.duration}
            · {Math.round(item.duration)}s{/if}
        </div>
      </button>
    {/each}
  </div>
{:else if !searchLoading && searchQuery}
  <div class="search-empty mono">{t("lyrics.noResults")}</div>
{/if}

<style>
  .search-input-row {
    display: flex;
    gap: 8px;
  }
  .search-input {
    flex: 1;
    background: var(--bg-raise);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 7px 10px;
    color: var(--text);
    font-size: 11px;
    letter-spacing: 0.05em;
    outline: none;
    transition: border-color 0.15s;
  }
  .search-input:focus {
    border-color: var(--accent);
  }
  .search-results {
    max-height: 300px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .search-item {
    background: none;
    border: none;
    text-align: left;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background 0.12s;
  }
  .search-item:hover {
    background: var(--bg-raise);
  }
  .search-item-track {
    font-size: 13px;
    margin-bottom: 2px;
  }
  .search-item-artist {
    font-size: 10px;
    color: var(--faint);
    letter-spacing: 0.05em;
  }
  .search-item-meta {
    font-size: 9px;
    color: var(--dim);
    letter-spacing: 0.05em;
    margin-top: 2px;
  }
  .search-empty {
    text-align: center;
    color: var(--faint);
    font-size: 11px;
    padding: 20px 0;
  }

  .btn-primary {
    background: var(--accent);
    border: none;
    border-radius: var(--radius-sm);
    color: var(--bg);
    padding: 7px 16px;
    font-size: 10.5px;
    letter-spacing: 0.06em;
    cursor: pointer;
    transition: opacity 0.15s, transform 0.1s;
  }
  .btn-primary:hover:not(:disabled) {
    opacity: 0.9;
  }
  .btn-primary:active:not(:disabled) {
    transform: scale(0.97);
  }
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
