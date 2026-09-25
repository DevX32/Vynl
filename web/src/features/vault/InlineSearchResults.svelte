<script lang="ts">
  import type { SearchResult } from "../../lib/types";
  import { Music, Plus, Check } from "lucide-svelte";
  import { fmtDuration } from "../../lib/format";
  import { t } from "@lib/i18n";

  let {
    query = "",
    results = [],
    loading = false,
    error = null,
    addedIds = new Set(),
    onSelect,
  }: {
    query?: string;
    results: SearchResult[];
    loading: boolean;
    error: string | null;
    addedIds: Set<string>;
    onSelect: (result: SearchResult) => void;
  } = $props();

  const q = $derived(query.trim());
  const addedCount = $derived(results.filter((r) => addedIds.has(r.id)).length);
</script>

<section class="panel" aria-live="polite">
  <header class="head">
    <span class="title mono">
      {#if loading}
        searching…
      {:else if error}
        search failed
      {:else if results.length > 0}
        {t("vault.searchResults", { n: results.length })}
        {#if addedCount > 0}
          · {addedCount} added{/if}
      {:else if q}
        no matches
      {:else}
        {t("vault.searchTitle")}
      {/if}
    </span>
    {#if q}
      <span class="query mono">“{q}”</span>
    {/if}
  </header>

  <div class="list">
    {#if loading}
      {#each Array(6) as _, i}
        <div class="skel" style:animation-delay="{i * 50}ms">
          <div class="skel-cover"></div>
          <div class="skel-meta">
            <div class="skel-line w60"></div>
            <div class="skel-line w40"></div>
          </div>
        </div>
      {/each}
    {:else if error}
      <div class="empty mono bad">{error}</div>
    {:else if results.length === 0}
      <div class="empty mono">
        {q ? `No results for “${q}”` : "Type a song, artist, or album"}
      </div>
    {:else}
      {#each results as r (r.id)}
        {@const added = addedIds.has(r.id)}
        <button
          type="button"
          class="row"
          class:added
          disabled={added}
          onclick={() => onSelect(r)}
        >
          <span class="cover">
            {#if r.cover}
              <img src={r.cover} alt="" loading="lazy" />
            {:else}
              <Music size={13} stroke-width={1.5} />
            {/if}
          </span>
          <span class="meta">
            <span class="name">{r.title}</span>
            <span class="artist"
              >{r.artist}{#if r.album} · {r.album}{/if}</span
            >
          </span>
          {#if r.duration}
            <span class="dur mono">{fmtDuration(r.duration)}</span>
          {/if}
          <span class="action" aria-hidden="true">
            {#if added}
              <Check size={13} stroke-width={2} />
            {:else}
              <Plus size={14} stroke-width={2} />
            {/if}
          </span>
        </button>
      {/each}
    {/if}
  </div>
</section>

<style>
  .panel {
    margin: 8px 0 4px;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    max-height: min(420px, 52vh);
  }

  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--line);
    flex-shrink: 0;
  }

  .title {
    font-size: 10.5px;
    letter-spacing: 0.08em;
    text-transform: capitalize;
    color: var(--dim);
  }

  .query {
    font-size: 10.5px;
    color: var(--faint);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: none;
  }

  .list::-webkit-scrollbar {
    width: 0;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 14px;
    background: none;
    border: none;
    border-bottom: 1px solid var(--line);
    color: var(--text);
    cursor: pointer;
    text-align: left;
    transition: background 0.1s;
  }

  .row:last-child {
    border-bottom: none;
  }

  .row:hover:not(:disabled) {
    background: color-mix(in srgb, var(--bg) 55%, transparent);
  }

  .row:disabled {
    cursor: default;
    opacity: 0.55;
  }

  .cover {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    background: var(--bg);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    flex-shrink: 0;
    color: var(--faint);
  }

  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .name {
    font-size: 13px;
    font-weight: 500;
    line-height: 1.3;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .artist {
    font-size: 11px;
    color: var(--dim);
    line-height: 1.35;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dur {
    flex-shrink: 0;
    font-size: 11px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }

  .action {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
    color: var(--faint);
    transition:
      background 0.15s,
      color 0.15s;
  }

  .row:hover:not(:disabled) .action {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .row.added .action {
    color: var(--accent);
  }

  .empty {
    padding: 28px 16px;
    text-align: center;
    font-size: 11.5px;
    color: var(--faint);
  }

  .empty.bad {
    color: var(--red);
  }

  .skel {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--line);
  }

  .skel-cover {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
    background: linear-gradient(
      90deg,
      var(--bg) 25%,
      rgba(255, 255, 255, 0.04) 50%,
      var(--bg) 75%
    );
    background-size: 200% 100%;
    animation: shimmer 1.5s infinite;
  }

  .skel-meta {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .skel-line {
    height: 9px;
    border-radius: 4px;
    background: linear-gradient(
      90deg,
      var(--bg) 25%,
      rgba(255, 255, 255, 0.04) 50%,
      var(--bg) 75%
    );
    background-size: 200% 100%;
    animation: shimmer 1.5s infinite;
  }

  .skel-line.w60 {
    width: 60%;
  }

  .skel-line.w40 {
    width: 40%;
  }

  @keyframes shimmer {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }
</style>
