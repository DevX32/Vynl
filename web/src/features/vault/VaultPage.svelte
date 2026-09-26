<script lang="ts">
  import { onMount } from "svelte";
  import type {
    Collection,
    DownloadEvent,
    DownloadSummary,
    SearchCandidate,
    SearchResult,
    TrackProgress,
  } from "../../lib/types";
  import { ChevronLeft, ChevronRight } from "lucide-svelte";
  import { vynl } from "../../lib/vynl";
  import { resolveCollectionWithPlugins } from "@lib/plugins/providers";
  import { getCurrentSettings } from "@state/settings.svelte";
  import { fmtDuration, totalSeconds } from "../../lib/format";
  import { t } from "@lib/i18n";
  import {
    getCurrentPlaylists,
    createPlaylist,
    addToPlaylist,
    refreshPlaylists,
  } from "@state/playlists.svelte";
  import TrackRow from "./TrackRow.svelte";
  import VaultHero from "./VaultHero.svelte";
  import TrackFilterBar from "./TrackFilterBar.svelte";
  import DownloadFooter from "./DownloadFooter.svelte";
  import InlineSearchResults from "./InlineSearchResults.svelte";
  import PlaylistPicker from "./PlaylistPicker.svelte";
  import Button from "../../components/Button.svelte";

  let { active = false }: { active?: boolean } = $props();
  const isActive = $derived(active);

  const settings = $derived(getCurrentSettings());

  let url = $state("");
  let resolving = $state(false);
  let collection = $state<Collection | null>(null);
  let progress = $state<Record<string, TrackProgress>>({});
  let summary = $state<DownloadSummary | null>(null);
  let running = $state(false);
  let reviewing = $state(false);
  let error = $state<string | null>(null);
  let matches = $state<Record<string, SearchCandidate[]>>({});
  let picks = $state<Record<string, number>>({});
  let doneIds = $state<string[]>([]);
  let syncOnlyNew = $state(true);
  let dlGeneration = $state(0);
  let searchResults = $state<SearchResult[]>([]);
  let searching = $state(false);
  let searchError = $state<string | null>(null);
  let searchGen = $state(0);
  let addedTrackIds = $state<Set<string>>(new Set());
  let showSearchResults = $state(false);
  let showPlaylistPicker = $state(false);
  let pendingPaths = $state<string[]>([]);
  let pendingCollectionTitle = $state("");
  let pickerBusy = $state(false);
  const PAGE_SIZE = 50;
  let page = $state(0);
  let trackFilter = $state<"all" | "queued" | "active" | "failed" | "done" | "skipped">("all");
  let filteredTracks = $derived.by(() => {
    const tracks = collection?.tracks ?? [];
    if (trackFilter === "all") return tracks;
    return tracks.filter((t) => {
      const s = progress[t.id]?.status;
      if (trackFilter === "done") return s === "done";
      if (trackFilter === "failed") return s === "error";
      if (trackFilter === "skipped") return s === "skipped";
      if (trackFilter === "active") return s === "searching" || s === "downloading" || s === "processing";
      return s === "queued" || !s;
    });
  });
  let totalPages = $derived(
    Math.max(1, Math.ceil((filteredTracks.length) / PAGE_SIZE)),
  );
  let pagedTracks = $derived.by(() => {
    const start = page * PAGE_SIZE;
    return filteredTracks.slice(start, start + PAGE_SIZE);
  });

  let listEl = $state<HTMLDivElement>();

  $effect(() => {
    void page;
    listEl?.scrollTo({ top: 0 });
  });

  $effect(() => {
    const tp = totalPages;
    if (page >= tp) page = Math.max(0, tp - 1);
  });

  function handleEvent(e: DownloadEvent): void {
    if (e.type === "track") {
      progress = { ...progress, [e.payload.trackId]: e.payload };
    } else if (e.type === "summary") {
      summary = e.payload;
    } else if (e.type === "finished") {
      running = false;
    }
  }

  let lastDoneSync = 0;
  const DONE_SYNC_DEBOUNCE_MS = 2000;

  async function refreshDone(opts?: { force?: boolean }): Promise<void> {
    if (!collection) return;
    const now = Date.now();
    if (!opts?.force && now - lastDoneSync < DONE_SYNC_DEBOUNCE_MS) return;
    lastDoneSync = now;
    try {
      doneIds = await vynl.syncDone(collection);
    } catch (e) {
      console.warn("syncDone failed:", e);
    }
  }

  $effect(() => {
    void settings.filenamePattern;
    void settings.overwrite;
    if (isActive && collection && !running && !resolving) {
      void refreshDone();
    }
  });

  function dismissSearch(): void {
    searchGen++;
    searching = false;
    url = "";
    showSearchResults = false;
    searchResults = [];
    searchError = null;
  }

  $effect(() => {
    if (!url.trim() && showSearchResults) {
      dismissSearch();
    }
  });

  async function resolve(): Promise<void> {
    if (!url.trim() || resolving) return;
    const query = url.trim();
    searchGen++;
    searching = false;
    showSearchResults = false;
    searchResults = [];
    searchError = null;
    resolving = true;
    error = null;
    progress = {};
    summary = null;
    matches = {};
    picks = {};
    reviewing = false;
    page = 0;
    trackFilter = "all";
    collection = null;
    addedTrackIds = new Set();
    doneIds = [];
    try {
      const c = await resolveCollectionWithPlugins(query);
      collection = c;
      addedTrackIds = new Set(c.tracks.map((t) => t.id));
      await refreshDone({ force: true });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      resolving = false;
    }
  }

  async function openSearch(query: string): Promise<void> {
    const trimmed = query.trim();
    if (!trimmed) return;
    const gen = ++searchGen;
    showSearchResults = true;
    searching = true;
    searchError = null;
    try {
      const results = await vynl.searchSpotify(trimmed, 20);
      if (gen !== searchGen) return;
      searchResults = results;
    } catch (e) {
      if (gen !== searchGen) return;
      searchError = e instanceof Error ? e.message : String(e);
      searchResults = [];
    } finally {
      if (gen === searchGen) searching = false;
    }
  }

  function addTrack(result: SearchResult): void {
    if (addedTrackIds.has(result.id)) {
      dismissSearch();
      return;
    }
    if (collection?.tracks.some((t) => t.id === result.id)) {
      addedTrackIds = new Set([...addedTrackIds, result.id]);
      dismissSearch();
      return;
    }
    addedTrackIds = new Set([...addedTrackIds, result.id]);
    const track = {
      id: result.id,
      title: result.title,
      artist: result.artist,
      album: result.album,
      year: null,
      duration: result.duration,
      cover: result.cover,
      trackNumber: (collection?.tracks.length ?? 0) + 1,
      url: result.url,
    };
    if (collection) {
      collection = {
        ...collection,
        tracks: [...collection.tracks, track],
      };
    } else {
      collection = {
        kind: "track",
        id: `search-${Date.now()}`,
        title: t("vault.searchTitle"),
        cover: result.cover,
        tracks: [track],
      };
    }
    dismissSearch();
  }

  function initProgress(
    status: TrackProgress["status"],
  ): Record<string, TrackProgress> {
    return Object.fromEntries(
      (collection?.tracks ?? []).map((t) => [
        t.id,
        { trackId: t.id, status, percent: 0 },
      ]),
    );
  }

  async function findAndReview(): Promise<void> {
    if (!collection || running) return;
    running = true;
    error = null;
    summary = null;

    await refreshDone({ force: true });

    const tracksToSearch = collection.tracks.filter((t) => !doneIds.includes(t.id));
    const skippedIds = new Set(doneIds.filter((id) => collection!.tracks.some((t) => t.id === id)));

    const searchCollection = { ...collection, tracks: tracksToSearch };

    progress = {};
    for (const t of collection.tracks) {
      if (skippedIds.has(t.id)) {
        progress[t.id] = { trackId: t.id, status: "skipped", percent: 0 };
      } else {
        progress[t.id] = { trackId: t.id, status: "queued", percent: 0 };
      }
    }

    try {
      const list = await vynl.matches(searchCollection);
      matches = list;
      picks = {};
      reviewing = true;
      page = 0;
    } catch (e) {
      error = e instanceof Error ? e.message : t("vault.couldNotSearch");
      matches = {};
      reviewing = false;
    } finally {
      running = false;
    }
  }

  async function beginDownload(): Promise<void> {
    if (!collection || running) return;
    if (settings.confirmMatches && Object.keys(matches).length === 0) {
      await findAndReview();
      return;
    }
    await refreshDone({ force: true });
    await confirmDownload();
  }

  function buildSkipIds(tracks: Collection["tracks"], doneIds: string[]): string[] {
    return tracks.filter((t) => doneIds.includes(t.id)).map((t) => t.id);
  }

  function markQueuedAsError(
    current: Record<string, TrackProgress>,
    tracks: Collection["tracks"],
    msg: string,
  ): Record<string, TrackProgress> {
    const next = { ...current };
    for (const t of tracks) {
      if (next[t.id]?.status === "queued") {
        next[t.id] = { trackId: t.id, status: "error", percent: 0, message: msg };
      }
    }
    return next;
  }

  function buildRetryCollection(): Collection | null {
    if (!collection) return null;
    const failed = collection.tracks.filter(
      (t) => progress[t.id]?.status === "error",
    );
    if (failed.length === 0) return null;
    return { ...collection, tracks: failed };
  }

  async function confirmDownload(): Promise<void> {
    if (!collection || running) return;
    if (syncOnlyNew) {
      await refreshDone({ force: true });
    }
    const gen = ++dlGeneration;
    running = true;
    reviewing = false;
    error = null;
    summary = null;

    const skipIds = syncOnlyNew ? buildSkipIds(collection.tracks, doneIds) : [];
    const baseProgress = initProgress("queued");
    for (const id of skipIds)
      baseProgress[id] = { trackId: id, status: "skipped", percent: 0 };
    progress = baseProgress;
    const dlCollection = collection;
    try {
      await vynl.startDownload(dlCollection, {
        matches,
        picks,
        skipIds,
      });
      if (gen !== dlGeneration) return;
      await postDownload(dlCollection);
    } catch (e) {
      if (gen !== dlGeneration) return;
      error = e instanceof Error ? e.message : t("vault.downloadFailed");
    } finally {
      if (gen === dlGeneration) running = false;
    }
  }

  async function postDownload(dlCollection: Collection): Promise<void> {
    try {
      const paths = await vynl.downloadedPaths(dlCollection);
      const existingPaths = paths.filter((p) => {
        const name = p.split(/[/\\]/).pop() ?? "";
        return name && !name.startsWith(".");
      });
      if (existingPaths.length === 0) return;

      await refreshPlaylists();
      pendingPaths = existingPaths;
      pendingCollectionTitle = dlCollection.title;
      pickerBusy = false;
      showPlaylistPicker = true;
    } catch (e) {
      console.warn("postDownload playlist error:", e);
    }
  }

  function closePicker(): void {
    showPlaylistPicker = false;
    pendingPaths = [];
    pendingCollectionTitle = "";
    pickerBusy = false;
  }

  async function addToSelectedPlaylist(playlistId: string): Promise<void> {
    if (pickerBusy) return;
    pickerBusy = true;
    try {
      await addToPlaylist(playlistId, pendingPaths);
      await refreshPlaylists();
    } catch (e) {
      console.warn("addToPlaylist error:", e);
    }
    closePicker();
  }

  async function createAndAddPlaylist(name: string): Promise<void> {
    if (pickerBusy) return;
    pickerBusy = true;
    try {
      const pl = await createPlaylist(name.trim() || "Untitled");
      await addToPlaylist(pl.id, pendingPaths);
      await refreshPlaylists();
    } catch (e) {
      console.warn("createAndAddPlaylist error:", e);
    }
    closePicker();
  }


  const counts = $derived.by(() => {
    if (!collection) return { done: 0, skipped: 0, failed: 0, queued: 0, active: 0 };
    let done = 0,
      skipped = 0,
      failed = 0,
      queued = 0,
      active = 0;
    for (const t of collection.tracks) {
      const status = progress[t.id]?.status;
      if (status === "done") done++;
      else if (status === "skipped") skipped++;
      else if (status === "error") failed++;
      else if (status === "queued" || !status) queued++;
      else active++;
    }
    return { done, skipped, failed, queued, active };
  });

  async function retryFailed(): Promise<void> {
    if (!collection || running) return;
    const sub = buildRetryCollection();
    if (!sub) return;
    const subMatches = Object.fromEntries(
      sub.tracks
        .map((t) => [t.id, matches[t.id]])
        .filter(([, m]) => m && m.length > 0),
    );
    const gen = ++dlGeneration;
    running = true;
    reviewing = false;
    error = null;
    summary = null;
    {
      const next = { ...progress };
      for (const t of sub.tracks)
        next[t.id] = { trackId: t.id, status: "queued", percent: 0 };
      progress = next;
    }
    try {
      await vynl.startDownload(JSON.parse(JSON.stringify(sub)), {
        matches: JSON.parse(JSON.stringify(subMatches)),
        picks: JSON.parse(JSON.stringify(picks)),
        skipIds: [],
      });
    } catch (e) {
      if (gen !== dlGeneration) return;
      error = e instanceof Error ? e.message : t("vault.retryFailedError");
      progress = markQueuedAsError(progress, sub.tracks, error);
    } finally {
      if (gen === dlGeneration) running = false;
    }
  }

  async function cancel(): Promise<void> {
    ++dlGeneration;
    try {
      await vynl.cancelDownload();
    } catch (err) {
      console.warn("cancel download failed:", err);
    }
    running = false;
    reviewing = false;
  }

  function clearAll(): void {
    dismissSearch();
    collection = null;
    progress = {};
    summary = null;
    running = false;
    reviewing = false;
    error = null;
    matches = {};
    picks = {};
    doneIds = [];
    page = 0;
    trackFilter = "all";
    addedTrackIds = new Set();
    closePicker();
  }

  function totalDur(): number {
    return totalSeconds(collection?.tracks ?? []);
  }

  function handleFilterChange(filter: string): void {
    trackFilter = filter as typeof trackFilter;
    page = 0;
  }

  onMount(() => {
    const offDl = vynl.onDownloadEvent(handleEvent);
    if (collection && isActive && !running && !resolving) void refreshDone();
    const onFocus = (): void => {
      if (collection && isActive && !running && !resolving) void refreshDone();
    };
    const onVis = (): void => {
      if (
        document.visibilityState === "visible" &&
        collection &&
        isActive &&
        !running &&
        !resolving
      )
        void refreshDone();
    };
    window.addEventListener("focus", onFocus);
    document.addEventListener("visibilitychange", onVis);
    return () => {
      offDl();
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onVis);
    };
  });
</script>

<div class="page">
  <VaultHero
    bind:value={url}
    onResolve={resolve}
    onSearch={openSearch}
    onClear={dismissSearch}
    resolving={resolving}
    searching={searching}
  />

  {#if showSearchResults}
    <InlineSearchResults
      query={url}
      results={searchResults}
      loading={searching}
      error={searchError}
      addedIds={addedTrackIds}
      onSelect={addTrack}
    />
  {/if}

  {#if collection && !showSearchResults}
    <div class="coll-header">
      <div class="coll-meta">
        <div class="coll-title display">{collection.title}</div>
        <div class="coll-sub mono">
          {collection.tracks.length}
          {collection.tracks.length === 1 ? t("vault.track") : t("vault.tracks")}
          {#if totalDur() > 0} · {fmtDuration(totalDur())}{/if}
        </div>
      </div>
      <TrackFilterBar
        total={collection.tracks.length}
        {trackFilter}
        {counts}
        onFilterChange={handleFilterChange}
      />
      <button
        type="button"
        class="coll-clear mono"
        disabled={running}
        onclick={clearAll}
      >
        {t("vault.clear")}
      </button>
    </div>

    {#if reviewing && !running}
      <div class="review-note">
        <span class="sync-text mono">{t("vault.reviewNote")}</span>
        {#if counts.failed > 0}
          <Button variant="ghost" size="sm" onclick={retryFailed}>
            {t("vault.retryFailed", { n: counts.failed })}
          </Button>
        {/if}
      </div>
    {/if}

    <div class="tbl-head">
      <span class="h-num mono">#</span>
      <span class="h-cover"></span>
      <span class="h-title mono">{t("library.titleHeader")}</span>
      <span class="h-source mono">{t("library.sourceHeader")}</span>
      <span class="h-status mono">{t("library.statusHeader")}</span>
    </div>

    <div class="list" bind:this={listEl}>
      {#each pagedTracks as track (track.id)}
        <TrackRow
          {track}
          progress={progress[track.id]}
          candidates={matches[track.id]}
          picked={picks[track.id]}
          allowPick={reviewing && !running}
          onpick={(id, i) => (picks[id] = i)}
        />
      {/each}
    </div>

    {#if totalPages > 1}
      <div class="pagination">
        <button
          class="page-arrow"
          disabled={page === 0}
          onclick={() => (page = Math.max(0, page - 1))}
        >
          <ChevronLeft size={14} />
        </button>
        <div class="page-pill mono">
          <span class="page-cur">{page + 1}</span>
          <span class="page-sep">/</span>
          <span class="page-total">{totalPages}</span>
        </div>
        <button
          class="page-arrow"
          disabled={page >= totalPages - 1}
          onclick={() => (page = Math.min(totalPages - 1, page + 1))}
        >
          <ChevronRight size={14} />
        </button>
      </div>
    {/if}
  {/if}

  <DownloadFooter
    {collection}
    {running}
    {reviewing}
    {summary}
    {counts}
    totalDuration={totalDur()}
    onBeginDownload={beginDownload}
    onConfirmDownload={confirmDownload}
    onRetryFailed={retryFailed}
    onCancel={cancel}
    onDone={clearAll}
  />
</div>

{#if showPlaylistPicker}
  <PlaylistPicker
    suggestedName={pendingCollectionTitle}
    trackCount={pendingPaths.length}
    playlists={getCurrentPlaylists()}
    busy={pickerBusy}
    onAddExisting={addToSelectedPlaylist}
    onCreate={createAndAddPlaylist}
    onSkip={closePicker}
  />
{/if}

<style>
  .page {
    --num-w: 32px;
    --cover-w: 36px;
    --status-w: 70px;
    --col-gap: 8px;
    --row-px: 10px;
    --row-py: 8px;
    --track-cols: var(--num-w) var(--cover-w) minmax(0, 1fr)
      minmax(140px, 1fr) var(--status-w);

    height: 100%;
    display: flex;
    flex-direction: column;
    margin: 0 -32px;
    padding: 0 48px 0 120px;
  }

  .coll-header {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 20px 0 16px;
  }

  .coll-clear {
    flex-shrink: 0;
    height: 28px;
    padding: 0 12px;
    border-radius: var(--radius-sm);
    background: transparent;
    border: 1px solid var(--line-strong);
    color: var(--dim);
    font-size: 11px;
    letter-spacing: 0.04em;
    cursor: pointer;
    transition:
      background 0.15s,
      color 0.15s;
  }

  .coll-clear:hover:not(:disabled) {
    color: var(--text);
    background: var(--bg-raise);
  }

  .coll-clear:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .coll-meta {
    flex: 1;
    min-width: 0;
  }

  .coll-title {
    font-size: 18px;
    font-weight: 600;
    line-height: 1.2;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-top: 2px;
  }

  .coll-sub {
    font-size: 11px;
    color: var(--dim);
    margin-top: 2px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .review-note {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 0;
  }

  .sync-text {
    font-size: 11px;
    color: var(--dim);
  }

  .tbl-head {
    display: grid;
    grid-template-columns: var(--track-cols);
    align-items: center;
    gap: var(--col-gap);
    padding: 8px var(--row-px);
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
    position: sticky;
    top: 0;
    z-index: 2;
    background: var(--bg);
    color: var(--text);
    font-size: 13px;
  }

  .h-num {
    text-align: right;
    font-size: 11px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }

  .h-title,
  .h-source,
  .h-status {
    min-width: 0;
    letter-spacing: 0.14em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .h-status {
    text-align: right;
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: none;
  }

  .list::-webkit-scrollbar {
    width: 0;
    height: 0;
  }

  .pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 8px 0;
    border-top: 1px solid var(--line);
  }

  .page-arrow {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    background: transparent;
    border: 1px solid var(--line-strong);
    color: var(--dim);
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s, color 0.15s;
  }

  .page-arrow:hover:not(:disabled) {
    background: var(--bg-raise);
    border-color: var(--line-strong);
    color: var(--text);
  }

  .page-arrow:active:not(:disabled) {
    background: rgba(255, 255, 255, 0.04);
  }

  .page-arrow:disabled {
    opacity: 0.25;
    cursor: default;
  }

  .page-pill {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 10px;
    height: 28px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line);
    font-size: 11px;
    letter-spacing: 0.04em;
  }

  .page-cur {
    color: var(--text);
    font-weight: 500;
  }

  .page-sep {
    color: var(--line-strong);
    margin: 0 1px;
  }

  .page-total {
    color: var(--faint);
  }

  @container player (max-width: 620px) {
    .page {
      margin: 0 -16px;
      padding: 0 16px;
      --num-w: 28px;
      --cover-w: 32px;
      --col-gap: 6px;
      --track-cols: var(--num-w) var(--cover-w) minmax(0, 1fr) var(--status-w);
    }

    .h-source {
      display: none;
    }

    .coll-header {
      gap: 10px;
      padding: 14px 0 12px;
    }
  }

  @container player (max-width: 420px) {
    .page {
      margin: 0 -10px;
      padding: 0 10px;
      --cover-w: 28px;
      --status-w: 64px;
      --col-gap: 4px;
      --row-px: 8px;
      --track-cols: var(--cover-w) minmax(0, 1fr) var(--status-w);
    }

    .h-num {
      display: none;
    }

    .coll-header {
      flex-wrap: wrap;
      gap: 8px;
      padding: 10px 0 8px;
    }
  }
</style>