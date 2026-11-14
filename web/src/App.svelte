<script lang="ts">
  import { onMount } from "svelte";
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
  import { refreshCloudStatus } from "@state/cloud.svelte";
  import { t } from "@lib/i18n";
  import { refreshLibrary } from "@state/library.svelte";
  import { playTrack } from "@lib/player.svelte";
  import { handleGlobalKey, getShowShortcutsOverlay, toggleShortcutsOverlay } from "@state/shortcuts.svelte";
  import { getEqOpen } from "@state/equalizer.svelte";
  import { getMobileSyncUiOpen } from "@state/mobile-sync.svelte";
  import { wasOnboarded } from "@lib/onboarding.svelte";
  import Onboarding from "./features/onboarding/Onboarding.svelte";
  import { initAppUpdate } from "@state/app-update.svelte";
  import { toasts } from "./lib/toast";
  import { toPascalCase } from "./lib/format";
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
  } from "@state/playlists.svelte";
  import ContextMenu, { type CtxEntry } from "./components/ContextMenu.svelte";
  import Dialog from "./components/Dialog.svelte";
  import Toast from "./components/Toast.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import PlayerPage from "./features/player/PlayerPage.svelte";
  import PlayerBar from "./features/player/PlayerBar.svelte";
  import FullscreenPlayer from "./features/player/FullscreenPlayer.svelte";
  import LibraryPage from "./features/playlists/LibraryPage.svelte";
  import PlaylistDetail from "./features/playlists/PlaylistDetail.svelte";
  import SettingsPage from "./features/settings/SettingsPage.svelte";
  import AccountModal from "./features/account/AccountModal.svelte";
  import EqualizerModal from "./features/settings/EqualizerModal.svelte";
  import MobileSyncModal from "./features/settings/MobileSyncModal.svelte";
  import VaultPage from "./features/vault/VaultPage.svelte";
  import HomePage from "./features/home/HomePage.svelte";
  import CataloguePage from "./features/catalogue/CataloguePage.svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import ShortcutsOverlay from "./components/ShortcutsOverlay.svelte";
  import { catalogueRoute, type Page } from "@lib/types";
  import { ImageOff, Pencil, Trash2 } from "lucide-svelte";

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
  let renameTarget = $state<{ id: string; name: string } | null>(null);
  let deleteTarget = $state<{ id: string; name: string } | null>(null);
  let createTarget = $state(false);
  let renameVal = $state("");
  let createVal = $state("");

  $effect(() => {
    if (renameTarget) renameVal = renameTarget.name;
  });

  $effect(() => {
    if (createTarget) createVal = "";
  });

  $effect(() => {
    const enabled = getCurrentSettings().discordRpc;
    if (!enabled) void vynl.rpcUpdate(null);
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

  async function bootApp(): Promise<(() => void) | undefined> {
    setPlayHandler((id, queue) => playTrack(id, queue));
    await initSettings();
    onboarded = wasOnboarded();
    void vynl.rpcUpdate(null);
    await Promise.all([refreshPlaylists(), refreshLibrary(), restoreNowPlaying()]);
    hideSplash();
    const off = vynl.onDownloadEvent((e) => {
      if (e.type === "finished") {
        void refreshPlaylists();
        void refreshLibrary();
      }
    });
    const offUpdate = vynl.onUpdateStatus((s) => {
      if (s.available && !s.downloading && !s.ready && !s.error) {
        toasts.success(t("update.availableTitle"));
      }
      if (s.ready) {

      }
      if (s.error) {
        toasts.error(t("update.failedTitle"));
      }
    });
    void initAppUpdate();
    void (async () => {
      try {
        const cloud = await refreshCloudStatus();
        if (!cloud.signedIn) return;
        const result = await vynl.cloudSync();
        if (result.pulled > 0 || result.deleted > 0) {
          await refreshPlaylists();
        }
      } catch (e) {
        console.warn("cloud sync failed", e);
      }
    })();
    return () => { off(); offUpdate(); };
  }

  onMount(() => {
    if (isMini) {
      hideSplash(200);
      return () => {};
    }

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
      disposeMini?.();
      cleanup?.();
      offToast();
      setPlayHandler(null);
      window.removeEventListener("keydown", handleGlobalKey);
      document.removeEventListener("vynl:toggle-fullscreen", onFullscreenToggle);
    };
  });

  async function openPlaylist(id: string): Promise<void> {
    navigate("playlist");
    await selectPlaylist(id);
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

  async function confirmDeletePlaylist(target: { id: string; name: string }): Promise<void> {
    const wasViewing = page === "playlist" && getSelectedId() === target.id;
    try {
      const result = await deletePlaylist(target.id);
      await refreshLibrary();
      if (result.failed > 0) {
        toasts.error(t("playlists.deletePartial", { failed: result.failed }));
      } else {
        toasts.success(
          t("playlists.deleteDone", { name: toPascalCase(target.name), deleted: result.deleted }),
        );
      }
      if (wasViewing) navigate("library");
    } catch {
      toasts.error(t("playlists.deleteFailed"));
    }
    deleteTarget = null;
  }

  function confirmRenamePlaylist(): void {
    const tgt = renameTarget;
    if (tgt && renameVal.trim()) void renamePlaylist(tgt.id, renameVal.trim());
    renameTarget = null;
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

  async function removePlaylistCover(id: string): Promise<void> {
    try {
      await setPlaylistCover(id, null);
    } catch (err) {
      console.warn("remove playlist cover failed:", err);
      toasts.error(t("playlist.coverFailed"));
    }
  }

  function ctxItems(): CtxEntry[] {
    const m = ctxMenu;
    if (!m || m.type !== "playlist" || !m.playlistId || !m.playlistName) return [];
    const id = m.playlistId;
    const name = m.playlistName;
    const items: CtxEntry[] = [
      {
        label: t("contextMenu.rename"),
        icon: Pencil,
        action: () => {
          renameTarget = { id, name };
        },
      },
    ];

    if (getPlaylistCustomCover(id)) {
      items.push(
        { type: "separator" },
        {
          label: t("playlist.removeCover"),
          icon: ImageOff,
          action: () => void removePlaylistCover(id),
        },
      );
    }

    items.push(
      { type: "separator" },
      {
        label: t("contextMenu.delete"),
        icon: Trash2,
        danger: true,
        action: () => {
          deleteTarget = { id, name };
        },
      },
    );

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
  </div>

  <PlayerBar onOpenPlayer={openPlayer} fullscreen={showFullscreen} onToggleFullscreen={() => { showFullscreen = !showFullscreen; }} />
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

{#if getEqOpen()}
  <EqualizerModal />
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
    items={ctxItems()}
  />
{/if}

{#if deleteTarget}
  <Dialog
    open
    title={t("playlists.deleteTitle")}
    description={t("playlists.deleteBody", { name: toPascalCase(deleteTarget.name) })}
    confirmLabel={t("playlists.deleteConfirm")}
    danger
    onconfirm={() => {
      const target = deleteTarget;
      if (target) void confirmDeletePlaylist(target);
    }}
    oncancel={() => (deleteTarget = null)}
  />
{/if}

{#if renameTarget}
  <Dialog
    open
    title={t("playlists.renameTitle")}
    description={t("playlists.renameDesc")}
    confirmLabel={t("playlists.renameConfirm")}
    onconfirm={confirmRenamePlaylist}
    oncancel={() => (renameTarget = null)}
  >
    <input
      class="dlg-input"
      type="text"
      bind:value={renameVal}
      spellcheck={false}
      onkeydown={(e) => {
        if (e.key === "Enter") confirmRenamePlaylist();
      }}
    />
  </Dialog>
{/if}

{#if createTarget}
  <Dialog
    open
    title={t("playlists.newTitle")}
    description={t("playlists.newDesc")}
    confirmLabel={t("playlists.newConfirm")}
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

  @container player (max-width: 420px) {
    main {
      padding: 0 10px;
    }

    .page-wrap.bleed {
      margin: 0 -10px;
    }
  }
</style>