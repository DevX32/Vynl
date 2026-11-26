<script lang="ts">
  import { untrack } from "svelte";
  import { Search, LoaderCircle } from "lucide-svelte";
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
  let selectedId = $state<string | null>(null);
  let version = 0;
  let autoQuery = "";

  const trimmedQuery = $derived(query.trim());
  const selected = $derived(results.find((r) => r.id === selectedId) ?? null);

  $effect(() => {
    const id = getCurrentId();
    untrack(() => {
      const current = getCurrentTrack();
      const next = current ? `${current.title} ${current.artist}`.trim() : "";
      autoQuery = next;
      query = next;
      results = [];
      selectedId = null;
      error = null;
      if (id && next) void run(next, true);
    });
  });

  async function run(q = query, scoped = q.trim() === autoQuery.trim()): Promise<void> {
    const trimmed = q.trim();
    if (!trimmed) {
      results = [];
      selectedId = null;
      error = null;
      return;
    }
    const ver = ++version;
    loading = true;
    error = null;
    try {
      const current = getCurrentTrack();
      const data = await vynl.lyricsSearch({
        query: trimmed,
        ...(scoped && current ? { trackName: current.title, artistName: current.artist } : {}),
      });
      if (ver !== version) return;
      results = [...data].sort((a, b) => Number(Boolean(b.syncedLyrics)) - Number(Boolean(a.syncedLyrics)));
      selectedId = results[0]?.id ?? null;
    } catch (e) {
      if (ver !== version) return;
      results = [];
      selectedId = null;
      error = e instanceof Error && e.message ? e.message : t("lyrics.searchFailed");
    } finally {
      if (ver === version) loading = false;
    }
  }

  function pick(item: LrcSearchResult): void {
    selectedId = item.id === selectedId ? null : item.id;
  }

  function apply(item: LrcSearchResult): void {
    const text = item.syncedLyrics ?? item.plainLyrics ?? "";
    if (!text) return;
    const result: LyricsResult = { kind: lyricsKindOf(text), text, source: "remote" };
    setLyricsManual(result, toLyricLines(result));
    onClose();
  }

  function previewText(item: LrcSearchResult): string {
    const raw = item.syncedLyrics ?? item.plainLyrics ?? "";
    return raw
      .split(/\r?\n/)
      .map((line) => line.replace(/^\[[^\]]+\]\s*/g, "").trim())
      .filter(Boolean)
      .slice(0, 8)
      .join("\n");
  }

  function fmtDuration(seconds: number): string {
    if (!Number.isFinite(seconds) || seconds <= 0) return "";
    const total = Math.round(seconds);
    return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
  }

  function kindLabel(item: LrcSearchResult): string {
    if (item.instrumental && !item.syncedLyrics && !item.plainLyrics) return t("lyrics.instrumental");
    return item.syncedLyrics ? t("lyrics.synced") : t("lyrics.plain");
  }
</script>

<div class="wrap">
  <div class="search">
    <div class="search-icon" aria-hidden="true">
      {#if loading}
        <LoaderCircle size={14} stroke-width={1.8} class="spin" />
      {:else}
        <Search size={14} stroke-width={1.8} />
      {/if}
    </div>
    <input
      class="search-input mono"
      type="text"
      bind:value={query}
      placeholder={t("lyrics.searchPlaceholder")}
      onkeydown={(e) => e.key === "Enter" && run()}
      spellcheck="false"
      autocomplete="off"
    />
    <button class="search-go mono" type="button" onclick={() => run()} disabled={loading || !trimmedQuery}>
      {t("common.search")}
    </button>
  </div>

  <div class="body">
    {#if error}
      <div class="message error">
        <p class="message-title display">{t("lyrics.searchFailed")}</p>
        <p class="message-sub mono">{error}</p>
      </div>
    {:else if loading && results.length === 0}
      <div class="message">
        <p class="message-sub mono">{t("lyrics.searchingLyrics")}</p>
      </div>
    {:else if results.length === 0 && trimmedQuery}
      <div class="message">
        <p class="message-title display">{t("lyrics.noResults")}</p>
        <p class="message-sub mono">{t("catalogue.noResultsHint")}</p>
      </div>
    {:else if results.length === 0}
      <div class="message">
        <p class="message-sub mono">{t("lyrics.searchPlaceholder")}</p>
      </div>
    {:else}
      <div class="list">
        {#each results as item (item.id)}
          {@const active = item.id === selectedId}
          {@const text = item.syncedLyrics ?? item.plainLyrics ?? ""}
          <div class="result" class:active>
            <button class="result-row" type="button" onclick={() => pick(item)}>
              <div class="result-main">
                <div class="result-title display">{item.trackName || t("common.unknownArtist")}</div>
                <div class="result-meta mono">
                  {item.artistName || t("common.unknownArtist")}{#if item.albumName} · {item.albumName}{/if}
                </div>
              </div>
              <div class="result-tags mono">
                <span class="tag" class:synced={!!item.syncedLyrics}>{kindLabel(item)}</span>
                {#if fmtDuration(item.duration)}<span class="tag">{fmtDuration(item.duration)}</span>{/if}
              </div>
            </button>

            {#if active}
              <div class="preview">
                {#if text}
                  <pre class="preview-text mono">{previewText(item)}</pre>
                {:else}
                  <p class="preview-empty mono">{t("lyrics.instrumental")}</p>
                {/if}
                <div class="preview-actions">
                  <button class="action ghost mono" type="button" onclick={() => (selectedId = null)}>{t("common.cancel")}</button>
                  <button class="action primary mono" type="button" onclick={() => apply(item)} disabled={!text}>
                    {t("lyrics.apply")}
                  </button>
                </div>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-width: 0;
  }

  .search {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 10px;
    padding: 0 12px;
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
  }

  .search:focus-within {
    border-color: color-mix(in srgb, var(--accent) 42%, var(--line-strong));
  }

  .search-icon {
    display: inline-flex;
    color: var(--faint);
  }

  .search-icon :global(.spin) {
    animation: spin .9s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  .search-input {
    width: 100%;
    min-width: 0;
    padding: 11px 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font-size: 11px;
    outline: none;
  }

  .search-input::placeholder { color: var(--faint); }

  .search-go {
    appearance: none;
    border: 0;
    border-left: 1px solid var(--line);
    background: transparent;
    color: var(--dim);
    padding: 11px 0 11px 12px;
    font-size: 10px;
    letter-spacing: .08em;
    text-transform: uppercase;
    cursor: pointer;
  }

  .search-go:hover:not(:disabled) { color: var(--accent); }
  .search-go:disabled { opacity: .45; cursor: default; }

  .body {
    min-height: 220px;
    max-height: min(420px, 56vh);
    overflow: auto;
    scrollbar-width: none;
  }

  .body::-webkit-scrollbar {
    display: none;
  }

  .message {
    min-height: 220px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    text-align: center;
    padding: 24px;
  }

  .message-title {
    margin: 0;
    font-size: 16px;
    color: var(--text);
  }

  .message-sub {
    margin: 0;
    font-size: 10px;
    line-height: 1.6;
    color: var(--faint);
    max-width: 34ch;
  }

  .message.error .message-title { color: var(--red); }

  .list {
    display: flex;
    flex-direction: column;
  }

  .result {
    border-bottom: 1px solid var(--line);
  }

  .result:last-child { border-bottom: 0; }

  .result-row {
    width: 100%;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    padding: 12px 0;
    border: 0;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .result-row:hover .result-title,
  .result.active .result-title {
    color: var(--accent);
  }

  .result-main { min-width: 0; }

  .result-title {
    font-size: 14px;
    line-height: 1.3;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: color .15s;
  }

  .result-meta {
    margin-top: 4px;
    font-size: 10px;
    color: var(--faint);
    letter-spacing: .03em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .result-tags {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .tag {
    padding: 3px 7px;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    font-size: 9px;
    letter-spacing: .07em;
    text-transform: uppercase;
    color: var(--dim);
    white-space: nowrap;
  }

  .tag.synced {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 35%, transparent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }

  .preview {
    padding: 0 0 14px;
  }

  .preview-text {
    margin: 0;
    padding: 12px;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--bg-raise) 70%, transparent);
    color: var(--dim);
    font-size: 10px;
    line-height: 1.7;
    white-space: pre-wrap;
    max-height: 180px;
    overflow: auto;
    scrollbar-width: none;
  }

  .preview-text::-webkit-scrollbar {
    display: none;
  }

  .preview-empty {
    margin: 0;
    padding: 12px;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    color: var(--faint);
    font-size: 10px;
  }

  .preview-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 10px;
  }

  .action {
    appearance: none;
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--dim);
    padding: 7px 12px;
    font-size: 10px;
    letter-spacing: .06em;
    text-transform: uppercase;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .action:hover:not(:disabled) { color: var(--text); }
  .action:disabled { opacity: .45; cursor: default; }

  .action.primary {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }

  .action.primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
  }

  @media (prefers-reduced-motion: reduce) {
    .search-icon :global(.spin), .result-title, .action { animation: none !important; transition: none !important; }
  }
</style>
