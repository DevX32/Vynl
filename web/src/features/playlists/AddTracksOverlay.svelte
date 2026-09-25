<script lang="ts">
  import { Plus, X, Check, FolderOpen } from "lucide-svelte";
  import { fade, fly } from "svelte/transition";
  import { cubicIn, cubicOut } from "svelte/easing";
  import SearchInput from "../../components/SearchInput.svelte";
  import { t } from "@lib/i18n";
  import type { LibraryTrack } from "../../lib/types";

  let {
    library,
    addQuery,
    onQuery,
    onClose,
    onAddPaths,
    added,
  }: {
    library: LibraryTrack[];
    addQuery: string;
    onQuery: (q: string) => void;
    onClose: () => void;
    onAddPaths: (paths: string[]) => void;
    added: Set<string>;
  } = $props();

  let searchInput: SearchInput | undefined = $state();

  const tracks = $derived(
    [...(addQuery
      ? library.filter((t) =>
          `${t.title} ${t.artist} ${t.album}`
            .toLowerCase()
            .includes(addQuery.toLowerCase()),
        )
      : library
    )].sort((a, b) => a.title.localeCompare(b.title)),
  );

  $effect(() => {
    requestAnimationFrame(() => searchInput?.focus());
  });
</script>

<div
  class="add-overlay"
  role="dialog"
  tabindex="-1"
  aria-label={t("playlist.addTracks")}
  transition:fade={{ duration: 150 }}
  onkeydown={(e) => {
    if (e.key === "Escape") onClose();
  }}
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
>
  <div
    class="add-panel"
    in:fly={{ x: 28, duration: 220, easing: cubicOut }}
    out:fly={{ x: 28, duration: 160, easing: cubicIn }}
  >
<div class="add-head">
        <span class="display add-title">{t("playlist.addTracks")}</span>
        <button
        class="icon-btn"
        onclick={onClose}
        aria-label={t("titleBar.close")}>
        <X size={16} stroke-width={1.5} />
      </button>
      </div>
    <div class="add-search">
      <SearchInput
        bind:this={searchInput}
        value={addQuery}
        placeholder={t("playlist.searchLibrary")}
        oninput={(v) => onQuery(v)}
      />
    </div>

    <div class="add-body">
      {#if tracks.length === 0}
<div class="add-empty mono">
            <FolderOpen size={16} stroke-width={1.5} style="flex-shrink:0" />
            <span>{addQuery
              ? `no matches for "${addQuery}"`
              : t("playlist.emptyRip")}</span>
          </div>
      {:else}
        {#each tracks as track (track.id)}
          <div class="add-row">
            <button
              class="add-row-main"
              onclick={() => onAddPaths([track.path])}
              disabled={added.has(track.path)}
            >
              <span class="add-row-title display">{track.title}</span>
              <span class="add-row-artist mono">{track.artist}</span>
            </button>
            <button
              class="icon-btn mini"
              class:added={added.has(track.path)}
              onclick={() => onAddPaths([track.path])}
              disabled={added.has(track.path)}
              aria-label={t("playlist.add")}
            >
              {#if added.has(track.path)}
                <Check size={12} stroke-width={1.5} />
              {:else}
                <Plus size={12} stroke-width={1.5} />
              {/if}
            </button>
          </div>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .add-overlay {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    right: -32px;
    display: flex;
    justify-content: flex-end;
    z-index: 40;
  }

  .add-panel {
    width: 380px;
    max-width: 90%;
    height: 100%;
    background: var(--surface);
    border-left: 1px solid var(--line-strong);
    display: flex;
    flex-direction: column;
    padding: 20px 20px 0;
  }

  .add-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 14px;
  }

  .add-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 24px;
  }

  .add-search {
    flex-shrink: 0;
    margin-bottom: 14px;
  }

  .add-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: none;
    padding-bottom: 18px;
  }

  .add-body::-webkit-scrollbar {
    display: none;
  }

  .add-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 30px 0;
    text-align: center;
    color: var(--faint);
    font-size: 11.5px;
  }

  .add-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 6px;
    border-radius: var(--radius-sm);
    transition: background 0.12s;
  }

  .add-row:hover {
    background: var(--bg-raise);
  }

  .add-row-main {
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

  .add-row-main:disabled {
    cursor: default;
    opacity: 0.7;
  }

  .add-row-title {
    width: 100%;
    font-size: 11.5px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .add-row-artist {
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

  .icon-btn.mini:hover:not(:disabled) {
    color: var(--text);
    background: var(--bg-raise);
  }

  .icon-btn.mini.added {
    color: var(--green);
  }

  .icon-btn.mini:disabled {
    cursor: default;
  }
</style>