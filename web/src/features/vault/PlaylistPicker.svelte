<script lang="ts">
  import { untrack } from "svelte";
  import { Search, Plus, ListMusic, Check } from "lucide-svelte";
  import { toPascalCase } from "../../lib/format";

  type Playlist = { id: string; name: string; trackCount: number };

  let {
    suggestedName = "",
    trackCount = 0,
    playlists = [],
    busy = false,
    onAddExisting,
    onCreate,
    onSkip,
  }: {
    suggestedName?: string;
    trackCount?: number;
    playlists: Playlist[];
    busy?: boolean;
    onAddExisting: (playlistId: string) => void;
    onCreate: (name: string) => void;
    onSkip: () => void;
  } = $props();

  let query = $state("");
  let newName = $state(untrack(() => suggestedName));
  let creating = $state(false);
  let dialogEl = $state<HTMLDivElement>();
  let nameEl = $state<HTMLInputElement>();

  $effect(() => {
    dialogEl?.focus();
  });

  $effect(() => {
    if (creating) nameEl?.focus();
  });

  const norm = (s: string): string => s.trim().toLowerCase();
  const existingMatch = $derived(
    suggestedName
      ? playlists.find((p) => norm(p.name) === norm(suggestedName))
      : undefined,
  );

  const filtered = $derived.by(() => {
    const q = norm(query);
    const list = q
      ? playlists.filter((p) => norm(p.name).includes(q))
      : [...playlists];
    if (!existingMatch) return list;
    return [
      ...list.filter((p) => p.id === existingMatch.id),
      ...list.filter((p) => p.id !== existingMatch.id),
    ];
  });

  const canCreate = $derived(newName.trim().length > 0);

  function submitCreate(): void {
    if (!canCreate || busy) return;
    onCreate(newName.trim());
  }

  function handleKey(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      if (creating) {
        creating = false;
      } else {
        onSkip();
      }
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="overlay" onclick={onSkip} role="presentation">
  <div
    class="dialog"
    bind:this={dialogEl}
    onclick={(e) => e.stopPropagation()}
    onkeydown={handleKey}
    role="dialog"
    aria-modal="true"
    aria-label="Add to playlist"
    tabindex="-1"
  >
    <header class="head">
      <div class="head-text">
        <span class="title">Add to playlist</span>
        <span class="sub mono">
          {trackCount}
          {trackCount === 1 ? "track" : "tracks"}
          {#if suggestedName}
            · {toPascalCase(suggestedName)}{/if}
        </span>
      </div>
    </header>

    {#if playlists.length > 4}
      <div class="search">
        <Search size={13} stroke-width={1.5} />
        <input
          class="search-input"
          placeholder="Filter playlists"
          bind:value={query}
          spellcheck={false}
        />
      </div>
    {/if}

    <div class="list">
      {#each filtered as pl (pl.id)}
        <button
          class="item"
          class:match={existingMatch?.id === pl.id}
          disabled={busy}
          onclick={() => onAddExisting(pl.id)}
        >
          <span class="item-icon">
            {#if existingMatch?.id === pl.id}
              <Check size={13} />
            {:else}
              <ListMusic size={13} />
            {/if}
          </span>
          <span class="name">{toPascalCase(pl.name)}</span>
          {#if existingMatch?.id === pl.id}
            <span class="tag mono">same name</span>
          {/if}
          <span class="count mono">{pl.trackCount}</span>
        </button>
      {:else}
        <div class="empty mono">
          {query ? "No playlist matches that" : "No playlists yet"}
        </div>
      {/each}
    </div>

    <footer class="foot">
      {#if creating}
        <input
          class="name-input"
          bind:this={nameEl}
          bind:value={newName}
          placeholder="Playlist name"
          spellcheck={false}
          onkeydown={(e) => {
            if (e.key === "Enter") submitCreate();
          }}
        />
        <button class="btn primary" disabled={!canCreate || busy} onclick={submitCreate}>
          Create
        </button>
      {:else}
        <button class="btn primary" disabled={busy} onclick={() => (creating = true)}>
          <Plus size={13} stroke-width={2} />
          <span>New playlist</span>
        </button>
        <button class="btn ghost" disabled={busy} onclick={onSkip}>Skip</button>
      {/if}
    </footer>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    backdrop-filter: blur(4px);
  }

  .dialog {
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    width: 380px;
    max-height: 480px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
    outline: none;
  }

  .head {
    padding: 16px 18px 12px;
    border-bottom: 1px solid var(--line);
  }

  .head-text {
    min-width: 0;
  }

  .title {
    display: block;
    font-size: 15px;
    font-weight: 600;
  }

  .sub {
    display: block;
    font-size: 11px;
    color: var(--dim);
    margin-top: 3px;
    letter-spacing: 0.04em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 16px;
    height: 36px;
    border-bottom: 1px solid var(--line);
    color: var(--faint);
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    outline: none;
    font-size: 12px;
    color: var(--text);
  }

  .search-input::placeholder {
    color: var(--faint);
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

  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 9px 18px;
    background: none;
    border: none;
    border-bottom: 1px solid var(--line);
    color: var(--text);
    cursor: pointer;
    text-align: left;
    transition: background 0.1s;
  }

  .item:hover:not(:disabled) {
    background: var(--bg-raise);
  }

  .item:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .item-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: var(--faint);
  }

  .item.match .item-icon {
    color: var(--accent);
  }

  .name {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tag {
    flex-shrink: 0;
    font-size: 9.5px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--accent);
    background: var(--accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
    border-radius: var(--radius-sm);
    padding: 1px 6px;
  }

  .count {
    flex-shrink: 0;
    font-size: 11px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }

  .empty {
    padding: 22px 18px;
    text-align: center;
    font-size: 11.5px;
    color: var(--faint);
  }

  .foot {
    padding: 10px 18px;
    border-top: 1px solid var(--line);
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .name-input {
    flex: 1;
    min-width: 0;
    height: 32px;
    padding: 0 10px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line-strong);
    outline: none;
    font-size: 12px;
    color: var(--text);
  }

  .name-input:focus {
    border-color: var(--accent);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 32px;
    padding: 0 14px;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.03em;
    cursor: pointer;
    white-space: nowrap;
    transition:
      background 0.15s,
      border-color 0.15s,
      color 0.15s;
  }

  .btn:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .btn.primary {
    flex: 1;
    color: var(--accent);
    background: var(--accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
  }

  .btn.primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
  }

  .btn.ghost {
    background: var(--bg-raise);
    color: var(--dim);
    border: 1px solid var(--line-strong);
  }

  .btn.ghost:hover:not(:disabled) {
    color: var(--text);
  }
</style>
