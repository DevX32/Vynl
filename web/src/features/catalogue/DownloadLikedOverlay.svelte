<script lang="ts">
  import { Plus, Check, X, Download, LoaderCircle, FolderOpen } from "lucide-svelte";
  import { untrack } from "svelte";
  import { fade, fly } from "svelte/transition";
  import { cubicIn, cubicOut } from "svelte/easing";
  import { t } from "@lib/i18n";
  import { stripAlbumSuffix } from "@lib/format";
  import SearchInput from "@components/SearchInput.svelte";
  import TrackCover from "@components/TrackCover.svelte";
  import type { CatalogueTrack } from "@lib/types";

  let {
    tracks,
    busy = false,
    onClose,
    onSubmit,
  }: {
    tracks: readonly CatalogueTrack[];
    busy?: boolean;
    onClose: () => void;
    onSubmit: (picked: CatalogueTrack[], mode: "add" | "download") => void;
  } = $props();

  let mode = $state<"add" | "download">("add");

  let query = $state("");
  let selected = $state<Set<string>>(new Set());
  let searchInput: SearchInput | undefined = $state();

  let debouncedQuery = $state(untrack(() => query));
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    const value = query;
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      debouncedQuery = value;
      debounceTimer = undefined;
    }, 150);
    return () => clearTimeout(debounceTimer);
  });

  $effect(() => {
    requestAnimationFrame(() => searchInput?.focus());
  });

  const hits = $derived(
    debouncedQuery.trim()
      ? tracks.filter((track) =>
          `${track.title} ${track.artist} ${track.album?.name ?? ""}`
            .toLowerCase()
            .includes(debouncedQuery.trim().toLowerCase()),
        )
      : tracks,
  );

  const selectedCount = $derived(selected.size);
  const allShownSelected = $derived(
    hits.length > 0 && hits.every((track) => selected.has(track.id)),
  );

  function toggle(track: CatalogueTrack): void {
    const next = new Set(selected);
    if (next.has(track.id)) {
      next.delete(track.id);
    } else {
      next.add(track.id);
    }
    selected = next;
  }

  function toggleAllShown(): void {
    const next = new Set(selected);
    if (allShownSelected) {
      for (const track of hits) next.delete(track.id);
    } else {
      for (const track of hits) next.add(track.id);
    }
    selected = next;
  }

  function submit(): void {
    if (busy) return;
    const picked = tracks.filter((track) => selected.has(track.id));
    if (picked.length === 0) return;
    onSubmit(picked, mode);
  }
</script>

<div
  class="dl-overlay"
  role="dialog"
  tabindex="-1"
  aria-label={t("catalogue.bulkActions")}
  transition:fade={{ duration: 150 }}
  onkeydown={(e) => {
    if (e.key === "Escape") onClose();
  }}
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
>
  <div
    class="dl-panel"
    in:fly={{ x: 28, duration: 220, easing: cubicOut }}
    out:fly={{ x: 28, duration: 160, easing: cubicIn }}
  >
    <div class="dl-head">
      <span class="display dl-title">{t("catalogue.bulkActions")}</span>
      <button
        class="icon-btn close-btn"
        onclick={onClose}
        aria-label={t("titleBar.close")}
      >
        <X size={12} stroke-width={1.6} />
      </button>
    </div>

    <div class="dl-tabs" role="tablist">
      <button
        class="dl-tab"
        class:active={mode === "add"}
        role="tab"
        aria-selected={mode === "add"}
        onclick={() => (mode = "add")}
      >
        <Plus size={12} stroke-width={1.5} />
        <span>{t("catalogue.addToPlaylist")}</span>
      </button>
      <button
        class="dl-tab"
        class:active={mode === "download"}
        role="tab"
        aria-selected={mode === "download"}
        onclick={() => (mode = "download")}
      >
        <Download size={12} stroke-width={1.5} />
        <span>{t("catalogue.downloadToPlaylist")}</span>
      </button>
    </div>

    <div class="dl-search">
      <SearchInput
        bind:this={searchInput}
        value={query}
        placeholder={t("playlist.searchPlaceholder")}
        oninput={(v) => (query = v)}
      />
    </div>

    <div class="dl-tools">
      <button class="tool-btn" onclick={toggleAllShown}>
        {allShownSelected ? t("catalogue.clearSelection") : t("catalogue.selectAll")}
      </button>
      <span class="tool-count mono">
        {t("catalogue.selected", { n: selectedCount })}
      </span>
    </div>

    <div class="dl-body">
      {#if hits.length === 0}
        <div class="dl-empty mono">
          <FolderOpen size={16} stroke-width={1.5} style="flex-shrink:0" />
          <span>{t("catalogue.noResults")}</span>
        </div>
      {:else}
        {#each hits as track (track.id)}
          {@const isSelected = selected.has(track.id)}
          <div class="dl-row" class:on={isSelected}>
            <TrackCover src={track.cover} shadow="var(--shadow-sm)" />
            <button class="dl-row-main" onclick={() => toggle(track)}>
              <span class="dl-row-title display">{stripAlbumSuffix(track.title, track.album?.name)}</span>
              <span class="dl-row-artist mono">{track.artist}</span>
            </button>
            <button
              class="icon-btn mini"
              class:added={isSelected}
              onclick={() => toggle(track)}
              aria-label={isSelected ? t("common.close") : t("playlist.add")}
            >
              {#if isSelected}
                <Check size={12} stroke-width={1.5} />
              {:else}
                <Plus size={12} stroke-width={1.5} />
              {/if}
            </button>
          </div>
        {/each}
      {/if}
    </div>

    <div class="dl-foot">
      <button
        class="dl-btn primary"
        disabled={selectedCount === 0 || busy}
        onclick={submit}
      >
        {#if busy}
          <span class="spin"><LoaderCircle size={13} stroke-width={1.5} /></span>
        {:else if mode === "add"}
          <Plus size={13} stroke-width={1.5} />
        {:else}
          <Download size={13} stroke-width={1.5} />
        {/if}
        {mode === "add"
          ? t("catalogue.addN", { n: selectedCount })
          : t("catalogue.downloadN", { n: selectedCount })}
      </button>
    </div>
  </div>
</div>

<style>
  .dl-overlay {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    right: -32px;
    display: flex;
    justify-content: flex-end;
    z-index: 40;
  }

  .dl-panel {
    width: 380px;
    max-width: 90%;
    height: 100%;
    background: var(--surface);
    border-left: 1px solid var(--line-strong);
    display: flex;
    flex-direction: column;
    padding: 20px 20px 16px;
  }

  .dl-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 14px;
  }

  .dl-title {
    font-size: 22px;
    letter-spacing: -0.01em;
  }

  .close-btn {
    width: 24px;
    height: 24px;
    background: var(--surface);
    border: 1px solid var(--line);
    transform: rotate(45deg);
    transition:
    color 0.15s,
    border-color 0.15s,
    background 0.15s;
  }

  .close-btn:hover {
    background: var(--surface);
    border-color: var(--line-strong);
  }

  .close-btn :global(svg) {
    transform: rotate(-45deg);
  }

  .dl-tabs {
    flex-shrink: 0;
    display: flex;
    gap: 4px;
    margin-bottom: 10px;
    padding: 4px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
    background: var(--bg-raise);
  }

  .dl-tab {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 28px;
    border: none;
    border-radius: calc(var(--radius-sm) - 2px);
    background: transparent;
    color: var(--faint);
    font-size: 11px;
    letter-spacing: 0.02em;
    cursor: pointer;
    transition:
      background 0.12s,
      color 0.12s;
  }

  .dl-tab:hover {
    color: var(--dim);
  }

  .dl-tab.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }

  .dl-search {
    flex-shrink: 0;
    margin-bottom: 10px;
  }

  .dl-tools {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 8px;
    padding: 0 2px;
  }

  .tool-btn {
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--faint);
    cursor: pointer;
    padding: 0;
    border: none;
    background: none;
    transition: color 0.12s;
  }

  .tool-btn:hover {
    color: var(--text);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .tool-count {
    font-size: 10px;
    color: var(--faint);
    letter-spacing: 0.05em;
  }

  .dl-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding-bottom: 8px;
  }

  .dl-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 30px 0;
    color: var(--faint);
    font-size: 11.5px;
  }

  .dl-row {
    --cover-size: 36px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 6px;
    border-radius: var(--radius-sm);
    transition: background 0.12s;
  }

  .dl-row:hover {
    background: var(--bg-raise);
  }

  .dl-row.on {
    background: var(--accent-soft);
  }

  .dl-row-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
    background: none;
    border: none;
    padding: 0;
    text-align: left;
    cursor: pointer;
  }

  .dl-row-title {
    width: 100%;
    font-size: 11.5px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dl-row-artist {
    width: 100%;
    font-size: 10px;
    color: var(--dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .icon-btn.mini {
    width: 22px;
    height: 22px;
    border-radius: var(--radius-sm);
    background: none;
    border: none;
    color: var(--faint);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    cursor: pointer;
    transition:
    color 0.15s,
    background 0.15s;
  }

  .icon-btn.mini:hover {
    color: var(--text);
    background: var(--bg-raise);
  }

  .icon-btn.mini.added {
    color: var(--green);
  }

  .dl-foot {
    flex-shrink: 0;
    display: flex;
    gap: 8px;
    padding-top: 12px;
    border-top: 1px solid var(--line);
  }

  .dl-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    flex: 1;
    height: 32px;
    padding: 0 14px;
    border-radius: var(--radius-sm);
    font-size: 11.5px;
    font-weight: 500;
    letter-spacing: 0.03em;
    cursor: pointer;
    transition:
    background 0.15s,
    border-color 0.15s,
    color 0.15s;
    white-space: nowrap;
  }

  .dl-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .dl-btn.primary {
    color: var(--accent);
    background: var(--accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
  }

  .dl-btn.primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
  }

  .spin {
    display: inline-flex;
    animation: spin 0.85s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
