<script lang="ts">
  import { untrack } from "svelte";
  import {
    Link2,
    LoaderCircle,
    AlertCircle,
    X,
    Search,
    Plus,
    ListMusic,
    Check,
    ChevronLeft,
    Disc3,
  } from "lucide-svelte";
  import type {
    Collection,
    StreamImportResult,
    ImportProgressEvent,
  } from "@lib/types";
  import { vynl } from "@lib/vynl";
  import { friendlyVaultError } from "@lib/errors";
  import { t } from "@lib/i18n";
  import { toasts } from "@lib/toast";
  import {
    getCurrentPlaylists,
    refreshPlaylistMeta,
  } from "@state/playlists.svelte";

  let {
    onClose,
    onImported,
  }: {
    onClose: () => void;
    onImported: (result: StreamImportResult) => void;
  } = $props();

  type Phase = "link" | "destination" | "importing";
  type DestTab = "new" | "existing";
  type StageKey = "resolve" | "match" | "save";
  type StageStatus = "pending" | "active" | "done";
  interface StageView {
    key: StageKey;
    label: string;
    status: StageStatus;
    done: number;
    total: number;
    detail: string;
  }

  let phase = $state<Phase>("link");
  let url = $state("");
  let resolving = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let collection = $state<Collection | null>(null);
  let cancelRequested = $state(false);
  let resolveState = $state<ImportProgressEvent | null>(null);
  let matchState = $state<ImportProgressEvent | null>(null);
  let saveState = $state<ImportProgressEvent | null>(null);

  let tab = $state<DestTab>("new");
  let query = $state("");
  let newName = $state("");
  let briefCoverFailed = $state(false);

  const playlists = $derived(getCurrentPlaylists());

  $effect(() => {
    const off = vynl.onImportProgress((e) => {
      if (e.phase === "resolve" || e.phase === "covers") resolveState = e;
      else if (e.phase === "match") matchState = e;
      else if (e.phase === "save") saveState = e;
    });
    return () => off();
  });

  $effect(() => {
    if (phase === "destination") void refreshPlaylistMeta();
  });

  function looksValid(raw: string): boolean {
    const v = raw.trim().toLowerCase();
    if (
      v.includes("open.spotify.com") ||
      v.includes("spotify:") ||
      v.includes("spotify.link") ||
      v.includes("spotify.app.link")
    ) {
      return true;
    }
    return /youtube\.com\/.*list=|youtu\.be\/.*list=/.test(v);
  }

  async function resolveLink(): Promise<void> {
    const raw = url.trim();
    if (!raw || resolving) return;
    resolving = true;
    error = null;
    try {
      const c = await vynl.resolve(raw);
      if (c.kind !== "playlist") {
        error = t("moveToVynl.notPlaylist");
        return;
      }
      if (c.tracks.length === 0) {
        error = t("moveToVynl.empty");
        return;
      }
      collection = c;
      tab = "new";
      query = "";
      newName = untrack(() => c.title.trim());
      briefCoverFailed = false;
      phase = "destination";
    } catch (e) {
      error = friendlyVaultError(e, t("common.couldNotOpenLink"));
    } finally {
      resolving = false;
    }
  }

  async function doImport(
    playlistId: string | null,
    playlistName: string | null,
  ): Promise<void> {
    if (!collection || busy) return;
    busy = true;
    error = null;
    phase = "importing";
    cancelRequested = false;
    matchState = null;
    saveState = null;
    try {
      const result = await vynl.importStreamPlaylist(
        collection,
        playlistId,
        playlistName,
      );
      onImported(result);
    } catch (e) {
      if (!cancelRequested) {
        const message = friendlyVaultError(e, t("moveToVynl.importFailed"));
        toasts.error(message);
        console.warn("stream playlist import failed:", e);
      }
      busy = false;
      phase = "destination";
    }
  }

  async function cancelImport(): Promise<void> {
    if (cancelRequested) return;
    cancelRequested = true;
    try {
      await vynl.cancelStreamImport();
    } catch {
      cancelRequested = false;
    }
  }

  const norm = (s: string): string => s.trim().toLowerCase();
  const suggestedName = $derived(collection?.title ?? "");
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
  const trackLabel = (n: number): string =>
    n === 1 ? t("common.trackLc") : t("common.tracks");
  const tracksLabel = $derived(
    collection ? trackLabel(collection.tracks.length) : "",
  );

  function submitCreate(): void {
    const name = newName.trim();
    if (!name || busy) return;
    void doImport(null, name);
  }

  function backToLink(): void {
    phase = "link";
  }

  function handleKey(e: KeyboardEvent): void {
    if (e.key !== "Escape") return;
    e.stopPropagation();
    if (phase === "importing") return;
    onClose();
  }

  const importLabel = $derived(collection?.title ?? "");

  const stageViews = $derived.by((): StageView[] => {
    const resolved = collection !== null;
    const tracks = collection?.tracks.length ?? 0;
    const matchDone =
      matchState !== null &&
      matchState.total > 0 &&
      matchState.done >= matchState.total;
    const saveDone =
      saveState !== null &&
      saveState.total > 0 &&
      saveState.done >= saveState.total;

    const resolveStatus: StageStatus = resolved
      ? "done"
      : resolveState || resolving
        ? "active"
        : "pending";
    const resolveDetail = resolved
      ? `${tracks} ${trackLabel(tracks)}`
      : resolveState
        ? resolveState.total > 0
          ? `${resolveState.done} / ${resolveState.total}`
          : `${resolveState.done} ${t("common.tracks")}`
        : resolving
          ? "…"
          : "";

    const matchStatus: StageStatus =
      phase === "importing"
        ? matchDone
          ? "done"
          : "active"
        : matchDone
          ? "done"
          : "pending";
    const matchDetail =
      matchState && matchState.total > 0
        ? `${matchState.done} / ${matchState.total}`
        : matchDone && tracks > 0
          ? `${tracks} / ${tracks}`
          : "";

    const saveStatus: StageStatus = saveDone
      ? "done"
      : saveState || (phase === "importing" && matchDone)
        ? "active"
        : "pending";
    const saveDetail = saveState ? `${saveState.done} / ${saveState.total}` : "";

    return [
      {
        key: "resolve",
        label: t("moveToVynl.stageResolve"),
        status: resolveStatus,
        done: resolveState?.done ?? 0,
        total: resolveState?.total ?? 0,
        detail: resolveDetail,
      },
      {
        key: "match",
        label: t("moveToVynl.stageMatch"),
        status: matchStatus,
        done: matchState?.done ?? 0,
        total: matchState?.total ?? tracks,
        detail: matchDetail,
      },
      {
        key: "save",
        label: t("moveToVynl.stageSave"),
        status: saveStatus,
        done: saveState?.done ?? 0,
        total: saveState?.total ?? 3,
        detail: saveDetail,
      },
    ];
  });
</script>

<svelte:window onkeydown={handleKey} />

{#snippet stageList()}
  <ul class="stages" aria-label={t("moveToVynl.stages")}>
    {#each stageViews as stage (stage.key)}
      <li class="stage {stage.status}">
        <span class="stage-icon" aria-hidden="true">
          {#if stage.status === "done"}
            <Check size={13} stroke-width={2} />
          {:else if stage.status === "active"}
            <LoaderCircle size={13} stroke-width={2} class="spin" />
          {:else}
            <span class="stage-dot"></span>
          {/if}
        </span>
        <span class="stage-label">{stage.label}</span>
        <span class="stage-count mono">{stage.detail}</span>
        {#if stage.status === "active" && stage.total > 0}
          <span
            class="stage-bar"
            role="progressbar"
            aria-valuenow={stage.done}
            aria-valuemin={0}
            aria-valuemax={stage.total}
            aria-label={stage.label}
          >
            <span
              class="stage-fill"
              style:width={`${Math.min(100, Math.round((stage.done / stage.total) * 100))}%`}
            ></span>
          </span>
        {/if}
      </li>
    {/each}
  </ul>
{/snippet}

<div class="overlay" role="presentation">
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label={t("common.moveToVynl")}
    tabindex="-1"
  >
    <header class="head">
      <span class="title">{t("common.moveToVynl")}</span>
      <button
        class="close-btn"
        onclick={onClose}
        disabled={phase === "importing"}
        aria-label={t("common.close")}
      >
        <X size={12} stroke-width={1.6} />
      </button>
    </header>

    {#if phase === "importing"}
      <div class="importing">
        <LoaderCircle size={26} stroke-width={1.75} class="spin" />
        <p class="importing-label">{importLabel}</p>
        {@render stageList()}
        <button
          class="btn ghost"
          onclick={() => void cancelImport()}
          disabled={cancelRequested}
        >
          {t("common.cancel")}
        </button>
      </div>
    {:else if phase === "destination" && collection}
      <div class="dest">
        <div class="brief">
          {#if collection.cover && !briefCoverFailed}
            <img
              class="brief-cover"
              src={collection.cover}
              alt=""
              referrerpolicy="no-referrer"
              loading="lazy"
              onerror={() => (briefCoverFailed = true)}
            />
          {:else}
            <div class="brief-cover placeholder">
              <Disc3 size={18} stroke-width={1.5} />
            </div>
          {/if}
          <div class="brief-meta">
            <span class="brief-title display">{collection.title}</span>
            <span class="brief-sub mono">
              {collection.tracks.length}
              {tracksLabel}
              {#if collection.owner}
                · {collection.owner}
              {/if}
            </span>
          </div>
        </div>

        <div class="seg" role="tablist" aria-label={t("moveToVynl.destination")}>
          <button
            class="seg-btn"
            class:active={tab === "new"}
            role="tab"
            aria-selected={tab === "new"}
            onclick={() => (tab = "new")}
          >
            {t("common.newPlaylist")}
          </button>
          <button
            class="seg-btn"
            class:active={tab === "existing"}
            role="tab"
            aria-selected={tab === "existing"}
            onclick={() => (tab = "existing")}
          >
            {t("moveToVynl.existingPlaylist")}
          </button>
        </div>

        {#if tab === "new"}
          <div class="pane new-pane">
            <label class="field-label mono" for="moveToVynl-name">
              {t("common.playlistName")}
            </label>
            <input
              id="moveToVynl-name"
              class="name-input"
              type="text"
              bind:value={newName}
              placeholder={t("common.playlistName")}
              spellcheck={false}
              autocomplete="off"
              autocapitalize="off"
              onkeydown={(e) => {
                if (e.key === "Enter") submitCreate();
              }}
            />
            <button
              class="btn primary wide"
              disabled={!canCreate || busy}
              onclick={submitCreate}
            >
              <Plus size={14} stroke-width={2} />
              <span>
                {t("moveToVynl.createAndAdd", { n: collection.tracks.length })}
              </span>
            </button>
          </div>
        {:else}
          <div class="pane">
            <div class="search">
              <Search size={13} stroke-width={1.5} />
              <input
                class="search-input"
                type="text"
                placeholder={t("moveToVynl.filter")}
                bind:value={query}
                spellcheck={false}
                autocomplete="off"
                autocapitalize="off"
              />
            </div>
            <div class="list">
              {#each filtered as pl (pl.id)}
                <button
                  class="item"
                  class:match={existingMatch?.id === pl.id}
                  disabled={busy}
                  onclick={() => void doImport(pl.id, null)}
                >
                  <span class="item-icon">
                    {#if existingMatch?.id === pl.id}
                      <Check size={13} stroke-width={2} />
                    {:else}
                      <ListMusic size={13} />
                    {/if}
                  </span>
                  <span class="item-name">{pl.name}</span>
                  {#if existingMatch?.id === pl.id}
                    <span class="tag mono">{t("common.sameName")}</span>
                  {/if}
                  <span class="count mono">{pl.trackCount}</span>
                </button>
              {:else}
                <div class="empty mono">
                  {query.trim()
                    ? t("common.noPlaylistMatches")
                    : t("common.noPlaylistsYet")}
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <footer class="foot dest-foot">
          <button class="btn ghost" onclick={backToLink}>
            <ChevronLeft size={13} stroke-width={2} />
            <span>{t("common.back")}</span>
          </button>
          <button class="btn ghost" onclick={onClose}>
            {t("common.cancel")}
          </button>
        </footer>
      </div>
    {:else}
      <div class="body">
        <p class="description">{t("moveToVynl.description")}</p>
        {@render stageList()}
        <div class="input-wrap" class:warn={url.trim() !== "" && !looksValid(url)}>
          <Link2 size={15} stroke-width={1.75} />
          <input
            class="link-input"
            type="text"
            placeholder={t("moveToVynl.placeholder")}
            bind:value={url}
            disabled={resolving}
            spellcheck={false}
            autocomplete="off"
            autocapitalize="off"
            onkeydown={(e) => {
              if (e.key === "Enter") void resolveLink();
            }}
          />
        </div>
        {#if error}
          <div class="error" role="alert">
            <AlertCircle size={13} stroke-width={1.75} />
            <span>{error}</span>
          </div>
        {/if}
      </div>
      <footer class="foot">
        <button class="btn ghost" onclick={onClose} disabled={resolving}>
          {t("common.cancel")}
        </button>
        <button
          class="btn primary"
          disabled={resolving || url.trim().length === 0}
          onclick={() => void resolveLink()}
        >
          {#if resolving}
            <LoaderCircle size={14} stroke-width={2} class="spin" />
          {/if}
          <span>{t("moveToVynl.continue")}</span>
        </button>
      </footer>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.75);
    z-index: 100;
  }

  .dialog {
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    width: 400px;
    max-width: calc(100vw - 48px);
    max-height: calc(100vh - 48px);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    outline: none;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px;
    border-bottom: 1px solid var(--line);
    flex-shrink: 0;
  }

  .title {
    font-size: 14px;
    font-weight: 600;
  }

  .close-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line);
    color: var(--faint);
    cursor: pointer;
    transform: rotate(45deg);
    transition:
      color 0.15s,
      background 0.15s,
      border-color 0.15s;
  }

  .close-btn :global(svg) {
    transform: rotate(-45deg);
  }

  .close-btn:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--line-strong);
  }

  .close-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .body {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .description {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--dim);
  }

  .stages {
    list-style: none;
    margin: 0;
    padding: 12px 0 0;
    display: flex;
    flex-direction: column;
    gap: 9px;
    border-top: 1px solid var(--line);
  }

  .stage {
    display: grid;
    grid-template-columns: 18px 1fr auto;
    column-gap: 10px;
    align-items: center;
    color: var(--faint);
  }

  .stage.done {
    color: var(--dim);
  }

  .stage.active {
    color: var(--text);
  }

  .stage-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--faint);
  }

  .stage.done .stage-icon {
    color: var(--green);
  }

  .stage.active .stage-icon {
    color: var(--accent);
  }

  .stage-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--line-strong);
  }

  .stage-label {
    font-size: 12px;
    line-height: 18px;
  }

  .stage-count {
    font-size: 10px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }

  .stage.active .stage-count {
    color: var(--accent);
  }

  .stage-bar {
    grid-column: 2 / 4;
    height: 3px;
    border-radius: 2px;
    background: var(--bg-raise);
    overflow: hidden;
    margin-top: 2px;
  }

  .stage-fill {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 0.2s ease;
  }

  .input-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 40px;
    padding: 0 12px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line-strong);
    color: var(--faint);
    transition: border-color 0.15s;
  }

  .input-wrap.warn {
    border-color: color-mix(in srgb, var(--red) 45%, transparent);
  }

  .input-wrap:has(:disabled) {
    opacity: 0.6;
  }

  .link-input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    outline: none;
    font-size: 12px;
    color: var(--text);
  }

  .link-input:disabled {
    color: var(--dim);
    cursor: default;
  }

  .link-input::placeholder {
    color: var(--faint);
  }

  .error {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 9px 11px;
    border: 1px solid var(--danger-border);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--red) 6%, transparent);
    color: var(--red);
    font-size: 11px;
    line-height: 1.45;
  }

  .dest {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .brief {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px 6px;
  }

  .brief-cover {
    width: 48px;
    height: 48px;
    border-radius: var(--radius-sm);
    object-fit: cover;
    flex-shrink: 0;
    background: var(--bg-raise);
    border: 1px solid var(--line);
  }

  .brief-cover.placeholder {
    display: grid;
    place-items: center;
    color: var(--faint);
  }

  .brief-meta {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .brief-title {
    font-size: 13px;
    font-weight: 600;
    line-height: 1.35;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .brief-sub {
    font-size: 10.5px;
    color: var(--faint);
    letter-spacing: 0.02em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .seg {
    display: flex;
    gap: 4px;
    margin: 10px 16px 0;
    padding: 4px;
    background: var(--bg-raise);
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }

  .seg-btn {
    flex: 1;
    height: 30px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--dim);
    font-size: 11.5px;
    font-weight: 500;
    letter-spacing: 0.02em;
    cursor: pointer;
    transition: color 0.12s, background 0.12s;
  }

  .seg-btn:hover {
    color: var(--text);
  }

  .seg-btn.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.35);
  }

  .pane {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 16px 16px;
    min-height: 0;
  }

  .new-pane {
    padding-top: 14px;
  }

  .field-label {
    font-size: 10px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--faint);
  }

  .name-input {
    height: 36px;
    padding: 0 12px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line-strong);
    outline: none;
    font-size: 12.5px;
    color: var(--text);
    transition: border-color 0.15s;
  }

  .name-input:focus {
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
  }

  .name-input::placeholder {
    color: var(--faint);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 12px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line-strong);
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
    max-height: 190px;
    min-height: 70px;
    overflow-y: auto;
    scrollbar-width: none;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
  }

  .list::-webkit-scrollbar {
    width: 0;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    padding: 9px 12px;
    background: none;
    border: none;
    border-bottom: 1px solid var(--line);
    color: var(--text);
    cursor: pointer;
    text-align: left;
    transition: background 0.1s;
  }

  .item:last-child {
    border-bottom: none;
  }

  .item:hover:not(:disabled) {
    background: var(--bg-raise);
  }

  .item:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .item-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--faint);
    flex-shrink: 0;
  }

  .item.match .item-icon {
    color: var(--accent);
  }

  .item-name {
    flex: 1;
    min-width: 0;
    font-size: 12.5px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tag {
    flex-shrink: 0;
    font-size: 9px;
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
    padding: 20px 12px;
    text-align: center;
    font-size: 11px;
    color: var(--faint);
  }

  .dest-foot {
    justify-content: space-between;
  }

  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 10px 16px 14px;
    border-top: 1px solid var(--line);
    flex-shrink: 0;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    height: 34px;
    padding: 0 14px;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.03em;
    cursor: pointer;
    white-space: nowrap;
    transition: color 0.15s, background 0.15s, border-color 0.15s;
  }

  .btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .btn.primary {
    color: var(--accent);
    background: var(--accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
  }

  .btn.primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
  }

  .btn.ghost {
    background: transparent;
    color: var(--dim);
    border: 1px solid var(--line-strong);
  }

  .btn.ghost:hover:not(:disabled) {
    color: var(--text);
    background: var(--bg-raise);
  }

  .btn.wide {
    width: 100%;
  }

  .importing {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 28px 24px 30px;
    color: var(--accent);
    text-align: center;
  }

  .importing-label {
    margin: 0;
    max-width: 100%;
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .importing .stages {
    width: 100%;
    text-align: left;
  }

  .foot :global(.spin),
  .importing :global(.spin),
  .stage :global(.spin) {
    animation: spin 0.85s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
