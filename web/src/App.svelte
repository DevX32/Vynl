<script lang="ts">
  import { onMount } from "svelte";
  import { fade, fly } from "svelte/transition";
  import { cubicIn, cubicOut } from "svelte/easing";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";
  import { vynl } from "./lib/vynl";
  import {
    restoreNowPlaying,
    setPlayHandler,
    getCurrentTrack,
    getIsTrackPlaying,
  } from "@state/now-playing.svelte";
  import { sendMiniState } from "@lib/mini-sync";
  import MiniPlayer from "./features/player/MiniPlayer.svelte";
  import { initSettings, getCurrentSettings } from "@state/settings.svelte";
  import { flushCloudSyncPush, refreshSyncStatus } from "@state/sync.svelte";
  import { hydrateTaste } from "@state/taste.svelte";
  import { t } from "@lib/i18n";
  import { refreshLibrary } from "@state/library.svelte";
  import { playTrack } from "@lib/player.svelte";
  import { handleGlobalKey, getShowShortcutsOverlay, toggleShortcutsOverlay } from "@state/shortcuts.svelte";
  import { getMobileSyncUiOpen } from "@state/mobile-sync.svelte";
  import { wasOnboarded } from "@lib/onboarding.svelte";
  import Onboarding from "./features/onboarding/Onboarding.svelte";
  import { initAppUpdate } from "@state/app-update.svelte";
  import { toasts } from "./lib/toast";
  import {
    getCurrentPlaylists,
    getPlaylistCustomCover,
    getSelectedId,
    refreshPlaylists,
    selectPlaylist,
    renamePlaylist,
    deletePlaylist,
    setPlaylistCover,
    createPlaylist,
    reorderPlaylists,
    setCurrentPlaylist,
  } from "@state/playlists.svelte";
  import ContextMenu, { type CtxEntry } from "./components/ContextMenu.svelte";
  import Dialog from "./components/Dialog.svelte";
  import Toast from "./components/Toast.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import PlayerPage from "./features/player/PlayerPage.svelte";
  import PlayerBar from "./features/player/PlayerBar.svelte";
  import FullscreenPlayer from "./features/player/FullscreenPlayer.svelte";
  import TagEditorModal from "./components/TagEditorModal.svelte";
  import { getQueueOpen } from "@state/queue.svelte";
  import QueueList from "./features/queue/QueueList.svelte";
  import { getTagEditorOpen } from "@state/tag-editor.svelte";
  import LibraryPage from "./features/playlists/LibraryPage.svelte";
  import PlaylistDetail from "./features/playlists/PlaylistDetail.svelte";
  import SettingsPage from "./features/settings/SettingsPage.svelte";
  import AccountModal from "./features/account/AccountModal.svelte";
  import MobileSyncModal from "./features/settings/MobileSyncModal.svelte";
  import VaultPage from "./features/vault/VaultPage.svelte";
  import HomePage from "./features/home/HomePage.svelte";
  import CataloguePage from "./features/catalogue/CataloguePage.svelte";
  import MoveToVynlDialog from "./features/playlists/MoveToVynlDialog.svelte";
  import type { StreamImportResult } from "./lib/types";
  import TitleBar from "./components/TitleBar.svelte";
  import ShortcutsOverlay from "./components/ShortcutsOverlay.svelte";
  import { catalogueRoute, type Page } from "@lib/types";
  import { normalizeCoverImage } from "@lib/cover-image";
  import { blobSrc } from "@lib/format";
  import { Disc3, Image, ImageOff, Pencil, Trash2, X } from "lucide-svelte";

  const isMini =
    typeof window !== "undefined" && window.location.hash.startsWith("#mini");

  let page = $state<Page>("home");
  function loadSidebarOpen(): boolean {
    if (typeof localStorage === "undefined") return true;
    return localStorage.getItem("vynl.sidebarOpen") !== "false";
  }

  let sidebarOpen = $state(loadSidebarOpen());
  let showFullscreen = $state(false);
  let onboarded = $state(true);
  let accountOpen = $state(false);

  const history = $state<Page[]>(["home"]);
  let historyIdx = $state(0);

  function navigate(p: Page): void {
    if (history[historyIdx] === p) return;
    history.splice(historyIdx + 1);
    history.push(p);
    historyIdx = history.length - 1;
    page = p;
  }

  function goBack(): void {
    if (historyIdx <= 0) return;
    historyIdx--;
    page = history[historyIdx];
  }

  function goForward(): void {
    if (historyIdx >= history.length - 1) return;
    historyIdx++;
    page = history[historyIdx];
  }

  let ctxMenu = $state<{
    x: number;
    y: number;
    playlistId?: string;
    playlistName?: string;
    type: "playlist" | "rail";
  } | null>(null);
  let moveToVynlOpen = $state(false);
  let createTarget = $state(false);
  let createVal = $state("");
  let editTarget = $state<{ id: string; name: string; cover: string | null } | null>(null);
  let editRename = $state("");
  let editBusy = $state(false);
  let editDeleteConfirm = $state(false);
  let editCoverInput: HTMLInputElement | undefined = $state();

  $effect(() => {
    if (createTarget) createVal = "";
  });

  $effect(() => {
    const enabled = getCurrentSettings().discordRpc;
    if (!enabled) void vynl.rpcUpdate(null).catch(() => {});
  });

  $effect(() => {
    if (isMini) return;
    const track = getCurrentTrack();
    const playing = getIsTrackPlaying();
    sendMiniState({
      track: track
        ? {
            id: track.id,
            path: track.path,
            title: track.title,
            artist: track.artist,
            album: track.album,
            cover: track.cover,
            duration: track.duration,
          }
        : null,
      playing,
    });
  });

  const playlists = $derived(getCurrentPlaylists());
  const catalogue = $derived(catalogueRoute(page));

  function setSidebarOpen(open: boolean): void {
    sidebarOpen = open;
    localStorage.setItem("vynl.sidebarOpen", String(open));
  }

  const SPLASH_MIN_MS = 2500;
  const splashReady = Date.now();

  function hideSplash(minMs: number = SPLASH_MIN_MS): void {
    const elapsed = Date.now() - splashReady;
    const wait = Math.max(0, minMs - elapsed);
    setTimeout(() => {
      const splash = document.getElementById("splash");
      if (splash) {
        splash.classList.add("hide");
        setTimeout(() => splash.remove(), 700);
      }
    }, wait);
  }

  const handledConfirmTokens = new Set<string>();

  async function handleConfirmUrl(url: string): Promise<void> {
    if (!url.startsWith("vynl://confirm")) return;
    const raw = url.replace(/^vynl:\/\/confirm[?&]/, "");
    const params = new URLSearchParams(raw);
    const token = params.get("token_hash") ?? params.get("token");
    if (!token || handledConfirmTokens.has(token)) return;
    handledConfirmTokens.add(token);
    const type = params.get("type") ?? "signup";
    try {
      const ok = await vynl.syncConfirmEmail(token, type);
      if (ok) toasts.success(t("account.emailConfirmed"));
      else toasts.error(t("account.emailConfirmFailed"));
    } catch (e) {
      handledConfirmTokens.delete(token);
      toasts.error(t("account.emailConfirmFailed"));
      console.warn("email confirmation failed", e);
    }
  }

  async function bootApp(): Promise<(() => void) | undefined> {
    setPlayHandler((id, queue) => playTrack(id, queue));
    onboarded = wasOnboarded();
    try {
      await initSettings();
      void vynl.rpcUpdate(null).catch(() => {});
      await Promise.all([
        refreshPlaylists().catch((e) => console.warn("playlist refresh failed:", e)),
        refreshLibrary().catch((e) => console.warn("library refresh failed:", e)),
        restoreNowPlaying().catch((e) => console.warn("restore failed:", e)),
      ]);
    } catch (e) {
      console.warn("boot failed", e);
    } finally {
      hideSplash();
    }
    const off = vynl.onDownloadEvent((e) => {
      if (e.type === "finished") {
        void refreshPlaylists();
        void refreshLibrary();
      }
    });
    void initAppUpdate();
    void (async () => {
      try {
        const session = await refreshSyncStatus();
        if (!session.signedIn) return;
        await hydrateTaste();
      } catch (e) {
        console.warn("sync status failed", e);
      }
    })();
    return () => { off(); };
  }

  onMount(() => {
    if (isMini) {
      hideSplash(200);
      return () => {};
    }

    void getCurrentWindow().show().catch(() => {});

    let offDeepLink: (() => void) | undefined;
    let deepLinkDisposed = false;
    void (async () => {
      try {
        const url = await vynl.takePendingConfirmUrl();
        if (url) await handleConfirmUrl(url);
        const urls = await getCurrent();
        for (const startUrl of urls ?? []) await handleConfirmUrl(startUrl);
      } catch (e) {
        console.warn("pending confirm failed", e);
      }
      try {
        const unlisten = await onOpenUrl((urls) => {
          for (const url of urls) void handleConfirmUrl(url);
        });
        if (deepLinkDisposed) unlisten();
        else offDeepLink = unlisten;
      } catch (e) {
        console.warn("deep link listener unavailable", e);
      }
    })();

    const offToast = vynl.onToast((summary) => {
      if (summary.cancelled) {
        toasts.info(t("vault.downloadCancelled"));
      } else if (summary.failed > 0) {
        toasts.warning(
          t("vault.downloadPartial", {
            done: summary.done,
            failed: summary.failed,
          }),
        );
      } else {
        toasts.success(t("vault.downloadComplete", { n: summary.done }));
      }
    });
    window.addEventListener("keydown", handleGlobalKey);
    window.addEventListener("pagehide", flushCloudSyncPush);
    const onFullscreenToggle = (): void => { showFullscreen = !showFullscreen; };
    document.addEventListener("vynl:toggle-fullscreen", onFullscreenToggle);
    let cleanup: (() => void) | undefined;
    let disposeMini: (() => void) | undefined;
    let disposed = false;
    void import("@state/mini-bridge.svelte").then((m) => {
      if (disposed) {
        m.initMiniBridge()();
      } else {
        disposeMini = m.initMiniBridge();
      }
    });
    void bootApp()
      .then((c) => {
        if (disposed) {
          c?.();
        } else {
          cleanup = c;
        }
      })
      .catch(console.error);
    return () => {
      disposed = true;
      deepLinkDisposed = true;
      offDeepLink?.();
      disposeMini?.();
      cleanup?.();
      offToast();
      setPlayHandler(null);
      window.removeEventListener("keydown", handleGlobalKey);
      window.removeEventListener("pagehide", flushCloudSyncPush);
      document.removeEventListener("vynl:toggle-fullscreen", onFullscreenToggle);
    };
  });

  async function openPlaylist(id: string): Promise<void> {
    navigate("playlist");
    await selectPlaylist(id);
  }

  function onMovedToVynl(result: StreamImportResult): void {
    moveToVynlOpen = false;
    setCurrentPlaylist(result.playlist);
    void refreshPlaylists();
    navigate("playlist");
    const name = result.playlist.name;
    if (result.unmatched.length === 0) {
      toasts.success(t("moveToVynl.done", { n: result.matched, name }));
    } else {
      toasts.warning(
        t("moveToVynl.partial", {
          n: result.matched,
          total: result.total,
          missed: result.unmatched.length,
          name,
        }),
      );
    }
  }

  function newPlaylist(): void {
    navigate("library");
    void selectPlaylist(null);
    createTarget = true;
  }

  function openPlayer(): void {
    navigate("player");
  }

  function openSettings(): void {
    navigate("settings");
  }

  function openAccount(): void {
    accountOpen = true;
  }

  function openVault(): void {
    navigate("vault");
  }

  function openLibrary(): void {
    navigate("library");
    void selectPlaylist(null);
  }

  function openCatalogue(): void {
    navigate("catalogue");
  }

  function openLikedSongs(): void {
    navigate("catalogue:liked");
  }

  function openHome(): void {
    navigate("home");
  }

  function showCtx(e: MouseEvent, id: string, name: string): void {
    e.preventDefault();
    e.stopPropagation();
    ctxMenu = {
      x: e.clientX,
      y: e.clientY,
      playlistId: id,
      playlistName: name,
      type: "playlist",
    };
  }

  function showRailCtx(e: MouseEvent): void {
    if ((e.target as HTMLElement).closest(".rail-item")) return;
    e.preventDefault();
    ctxMenu = { x: e.clientX, y: e.clientY, type: "rail" };
  }

  async function confirmCreatePlaylist(): Promise<void> {
    if (!createVal.trim()) return;
    createTarget = false;
    try {
      const pl = await createPlaylist(createVal.trim());
      await selectPlaylist(pl.id);
      navigate("playlist");
    } catch (err) {
      console.warn("create playlist failed:", err);
    }
  }

  function pickEditCover(): void {
    editCoverInput?.click();
  }

  async function onEditCoverFile(e: Event): Promise<void> {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file || !editTarget || editBusy) return;
    editBusy = true;
    try {
      const { ext, data } = await normalizeCoverImage(file);
      const savedPath = await vynl.savePlaylistCover(editTarget.id, ext, data);
      await setPlaylistCover(editTarget.id, savedPath);
      editTarget = { ...editTarget, cover: savedPath };
    } catch (err) {
      console.warn("set playlist cover failed:", err);
      toasts.error(t("playlist.coverFailed"));
    } finally {
      editBusy = false;
    }
  }

  async function saveEditRename(): Promise<void> {
    if (!editTarget || editBusy) return;
    const next = editRename.trim();
    if (!next || next === editTarget.name) {
      editTarget = null;
      return;
    }
    editBusy = true;
    try {
      await renamePlaylist(editTarget.id, next);
      editTarget = null;
    } catch (err) {
      console.warn("rename playlist failed:", err);
      toasts.error(t("playlist.renameFailed"));
    } finally {
      editBusy = false;
    }
  }

  async function removeEditCover(): Promise<void> {
    if (!editTarget || editBusy) return;
    editBusy = true;
    try {
      await setPlaylistCover(editTarget.id, null);
      editTarget = { ...editTarget, cover: null };
    } catch (err) {
      console.warn("remove playlist cover failed:", err);
      toasts.error(t("playlist.coverFailed"));
    } finally {
      editBusy = false;
    }
  }

  async function confirmEditDeletePlaylist(): Promise<void> {
    if (!editTarget || editBusy) return;
    editBusy = true;
    try {
      await deletePlaylist(editTarget.id);
      editDeleteConfirm = false;
      editTarget = null;
    } catch (err) {
      console.warn("delete playlist failed:", err);
      toasts.error(t("playlists.deleteFailed"));
    } finally {
      editBusy = false;
    }
  }

  function ctxItems(): CtxEntry[] {
    const m = ctxMenu;
    if (!m || m.type !== "playlist" || !m.playlistId || !m.playlistName) return [];
    const id = m.playlistId;
    const name = m.playlistName;
    const items: CtxEntry[] = [
      {
        label: t("playlist.editPlaylist"),
        icon: Pencil,
        action: () => {
          editTarget = { id, name, cover: getPlaylistCustomCover(id) ?? null };
          editRename = name;
          editDeleteConfirm = false;
        },
      },
    ];

    return items;
  }
</script>

{#if isMini}
  <MiniPlayer />
{:else if !onboarded}
  <Onboarding onComplete={() => (onboarded = true)} />
{:else}
<div class="shell">
  <TitleBar
    onBack={goBack}
    onForward={goForward}
    canBack={historyIdx > 0}
    canForward={historyIdx < history.length - 1}
    onHome={openHome}
    {sidebarOpen}
    onToggleSidebar={() => setSidebarOpen(!sidebarOpen)}
    onShowShortcuts={toggleShortcutsOverlay}
  />
  <div class="body">
    <Sidebar
      {page}
      {sidebarOpen}
      {playlists}
      selectedId={getSelectedId()}
      onLibrary={openLibrary}
      onCatalogue={openCatalogue}
      onPlayer={openPlayer}
      onVault={openVault}
      onSettings={openSettings}
      onAccount={openAccount}
      onNewPlaylist={newPlaylist}
      onMoveToVynl={() => (moveToVynlOpen = true)}
      onOpenPlaylist={(id) => void openPlaylist(id)}
      onOpenLikedSongs={openLikedSongs}
      onShowCtx={showCtx}
      onShowRailCtx={showRailCtx}
      onReorderPlaylists={(fromId, toId) => void reorderPlaylists(fromId, toId)}
    />

    <main>
      <div class="page-wrap" class:bleed={page === "player"}>
        {#if page === "home"}
          <div class="page-inner">
            <HomePage onNavigate={(p) => navigate(p)} />
          </div>
        {:else if page === "library"}
          <div class="page-inner">
            <LibraryPage active={true} />
          </div>
        {:else if catalogue}
          <div class="page-inner">
            <CataloguePage route={catalogue} onNavigate={(p) => navigate(p)} />
          </div>
        {:else if page === "playlist"}
          <div class="page-inner">
            <PlaylistDetail />
          </div>
        {:else if page === "player"}
          <div class="page-inner">
            <PlayerPage active={true} fullscreen={showFullscreen} />
          </div>
        {:else if page === "settings"}
          <div class="page-inner">
            <SettingsPage onOpenVault={openVault} />
          </div>
        {/if}
        <div class="page-inner" class:hidden={page !== "vault"}>
          <VaultPage active={page === "vault"} />
        </div>
      </div>
    </main>

    {#if getQueueOpen()}
      <QueueList />
    {/if}
  </div>

  <PlayerBar onOpenPlayer={openPlayer} fullscreen={showFullscreen} active={page === "player" || showFullscreen} onToggleFullscreen={() => { showFullscreen = !showFullscreen; }} />
</div>

{#if showFullscreen}
  <FullscreenPlayer onclose={() => { showFullscreen = false; }} />
{/if}

{#if getShowShortcutsOverlay()}
  <ShortcutsOverlay />
{/if}

{#if accountOpen}
  <AccountModal onclose={() => (accountOpen = false)} />
{/if}

{#if getTagEditorOpen()}
  <TagEditorModal />
{/if}

{#if getMobileSyncUiOpen()}
  <MobileSyncModal />
{/if}

{#if ctxMenu}
  <ContextMenu
    x={ctxMenu.x}
    y={ctxMenu.y}
    onclose={() => (ctxMenu = null)}
    onnewplaylist={ctxMenu.type === "rail" ? newPlaylist : undefined}
    onmovetovynl={ctxMenu.type === "rail" ? () => (moveToVynlOpen = true) : undefined}
    items={ctxItems()}
  />
{/if}

{#if createTarget}
  <Dialog
    open
    title={t("common.newPlaylistTitle")}
    description={t("playlists.newDesc")}
    confirmLabel={t("common.create")}
    onconfirm={confirmCreatePlaylist}
    oncancel={() => (createTarget = false)}
  >
    <input
      class="dlg-input"
      type="text"
      bind:value={createVal}
      spellcheck={false}
      onkeydown={(e) => {
        if (e.key === "Enter") void confirmCreatePlaylist();
      }}
    />
  </Dialog>
{/if}

{#if editTarget}
  <div
    class="backdrop"
    role="presentation"
    transition:fade={{ duration: 150 }}
    onclick={() => (editTarget = null)}
  >
    <div
      class="edit-panel"
      role="dialog"
      aria-modal="true"
      aria-label={t("playlist.editPlaylist")}
      tabindex="-1"
      in:fly={{ y: 8, duration: 180, easing: cubicOut }}
      out:fly={{ y: 8, duration: 140, easing: cubicIn }}
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => {
        if (e.key === "Escape") {
          e.preventDefault();
          editTarget = null;
        }
      }}
    >
      <div class="edit-head">
        <span class="edit-title">{t("playlist.editPlaylist")}</span>
        <button class="edit-close" onclick={() => (editTarget = null)} aria-label={t("common.cancel")}>
          <X size={12} stroke-width={1.6} />
        </button>
      </div>

      <div class="edit-hero">
        <div class="edit-cover">
          {#if editTarget.cover}
            <img class="edit-cover-img" use:blobSrc={editTarget.cover} alt="" loading="lazy" draggable="false" />
          {:else}
            <div class="edit-cover-placeholder"><Disc3 size={24} stroke-width={1} /></div>
          {/if}
          <div class="edit-cover-overlay">
            <button class="cover-action" onclick={pickEditCover} disabled={editBusy} aria-label={t("playlist.changeCover")}>
              <Image size={14} stroke-width={1.5} />
            </button>
            {#if editTarget.cover}
              <button class="cover-action" onclick={() => void removeEditCover()} disabled={editBusy} aria-label={t("playlist.removeCover")}>
                <ImageOff size={14} stroke-width={1.5} />
              </button>
            {/if}
          </div>
        </div>
        <div class="edit-field">
          <label class="edit-label" for="pl-rename-side">{t("playlists.renameTitle")}</label>
          <div class="edit-row">
            <input
              id="pl-rename-side"
              class="dlg-input"
              type="text"
              bind:value={editRename}
              spellcheck={false}
              onkeydown={(e) => {
                if (e.key === "Enter") void saveEditRename();
              }}
            />
            <button class="action-btn-sm" onclick={() => void saveEditRename()} disabled={editBusy || !editRename.trim()}>
              {t("common.save")}
            </button>
          </div>
          <button class="edit-action" onclick={() => (editDeleteConfirm = true)} disabled={editBusy}>
            <Trash2 size={14} stroke-width={1.5} />
            <span>{t("playlists.deleteTitle")}</span>
          </button>
        </div>
      </div>
      <input
        bind:this={editCoverInput}
        type="file"
        accept="image/*"
        class="cover-file-input"
        onchange={onEditCoverFile}
      />
    </div>
  </div>
{/if}

{#if editDeleteConfirm && editTarget}
  <Dialog
    open
    title={t("playlists.deleteTitle")}
    description={t("playlists.deleteBody", { name: editTarget.name })}
    confirmLabel={t("common.delete")}
    danger
    confirmDisabled={editBusy}
    onconfirm={() => void confirmEditDeletePlaylist()}
    oncancel={() => (editDeleteConfirm = false)}
  />
{/if}

{#if moveToVynlOpen}
  <MoveToVynlDialog
    onClose={() => (moveToVynlOpen = false)}
    onImported={onMovedToVynl}
  />
{/if}

<Toast />
{/if}

<style>
  .shell {
    height: 100vh;
    display: flex;
    flex-direction: column;
    container-type: inline-size;
    container-name: player;
  }

  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    position: relative;
  }

  main {
    flex: 1;
    min-height: 0;
    min-width: 0;
    padding: 0 32px;
  }

  .page-wrap {
    height: 100%;
    position: relative;
  }

  .page-wrap.bleed {
    margin: 0 -32px;
  }

  .page-inner {
    height: 100%;
  }

  .page-inner.hidden {
    display: none;
  }

  @container player (max-width: 620px) {
    main {
      padding: 0 16px;
    }

    .page-wrap.bleed {
      margin: 0 -16px;
    }
  }

  .backdrop {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 16px;
    background: rgba(0, 0, 0, 0.56);
    backdrop-filter: blur(8px);
  }

  .edit-panel {
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 16px;
    width: min(420px, 100%);
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.55), inset 0 1px 0 rgba(255, 255, 255, 0.03);
  }

  .edit-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .edit-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text);
    letter-spacing: -0.01em;
  }

  .edit-close {
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line);
    color: var(--faint);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transform: rotate(45deg);
  }

  .edit-close:hover {
    color: var(--text);
    border-color: var(--line-strong);
  }

  .edit-close :global(svg) {
    transform: rotate(-45deg);
  }

  .edit-hero {
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .edit-cover {
    position: relative;
    width: 112px;
    height: 112px;
    flex: 0 0 112px;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg-raise);
    border: 1px solid var(--line);
  }

  .edit-cover-img,
  .edit-cover-placeholder {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .edit-cover-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--faint);
  }

  .edit-cover-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 8px;
    background: rgba(0, 0, 0, 0.42);
    opacity: 0;
    transition: opacity 0.14s ease;
  }

  .edit-cover:hover .edit-cover-overlay,
  .edit-cover:focus-within .edit-cover-overlay {
    opacity: 1;
  }

  @media (hover: none) {
    .edit-cover-overlay {
      opacity: 1;
    }
  }

  .cover-action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: var(--radius-sm);
    background: rgba(18, 18, 18, 0.5);
    color: #fff;
    cursor: pointer;
    backdrop-filter: blur(6px);
  }

  .cover-action:hover:not(:disabled) {
    background: rgba(18, 18, 18, 0.72);
  }

  .cover-action:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .edit-hero .edit-field {
    flex: 1;
    min-width: 0;
  }

  .edit-field {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .edit-label {
    font-size: 11px;
    color: var(--dim);
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .edit-row {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .edit-row .dlg-input {
    flex: 1;
    min-width: 0;
  }

  .action-btn-sm {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line-strong);
    background: var(--bg-raise);
    color: var(--text);
    font: 500 12px inherit;
    cursor: pointer;
    white-space: nowrap;
  }

  .action-btn-sm:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .edit-action {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
    background: transparent;
    color: var(--text);
    font: 500 12px inherit;
    cursor: pointer;
  }

  .edit-action:hover:not(:disabled) {
    background: var(--bg-raise);
  }



  .edit-action:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .cover-file-input {
    display: none;
  }

  @container player (max-width: 420px) {
    main {
      padding: 0 10px;
    }

    .page-wrap.bleed {
      margin: 0 -10px;
    }
  }
</style>