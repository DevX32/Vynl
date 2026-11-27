<script lang="ts">
  import { onMount } from "svelte";
  import { attachConsole } from "@tauri-apps/plugin-log";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";
  import { vynl } from "./lib/vynl";
  import {
    restoreNowPlaying,
    setPlayHandler,
    getCurrentTrack,
    getIsTrackPlaying,
  } from "@state/now-playing.svelte";
  import { miniTrackSnapshot, sendMiniState } from "@lib/mini-sync";
  import MiniPlayer from "./features/player/MiniPlayer.svelte";
  import { initSettings, getCurrentSettings } from "@state/settings.svelte";
  import {
    flushCloudSyncPush,
    refreshSyncStatus,
    retryPendingSync,
  } from "@state/sync.svelte";
  import { hydrateTaste } from "@state/taste.svelte";
  import { t } from "@lib/i18n";
  import { refreshLibrary } from "@state/library.svelte";
  import { playTrack } from "@lib/playback.svelte";
  import {
    handleGlobalKey,
    getShowShortcutsOverlay,
    toggleShortcutsOverlay,
  } from "@state/shortcuts.svelte";
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
    reorderPlaylists,
    setCurrentPlaylist,
  } from "@state/playlists.svelte";
  import ContextMenu, { type CtxEntry } from "./components/ContextMenu.svelte";
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
  import PlaylistModals from "./features/playlists/PlaylistModals.svelte";
  import type { StreamImportResult } from "./lib/types";
  import TitleBar from "./components/TitleBar.svelte";
  import ShortcutsOverlay from "./components/ShortcutsOverlay.svelte";
  import { catalogueRoute, type Page } from "@lib/types";
  import { Pencil } from "lucide-svelte";

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
  let editTarget = $state<{
    id: string;
    name: string;
    cover: string | null;
  } | null>(null);

  $effect(() => {
    const enabled = getCurrentSettings().discordRpc;
    if (!enabled) void vynl.rpcUpdate(null).catch(() => {});
  });

  $effect(() => {
    if (isMini) return;
    sendMiniState(miniTrackSnapshot(getCurrentTrack(), getIsTrackPlaying()));
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
        refreshPlaylists().catch((e) =>
          console.warn("playlist refresh failed:", e),
        ),
        refreshLibrary().catch((e) =>
          console.warn("library refresh failed:", e),
        ),
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
    return () => {
      off();
    };
  }

  onMount(() => {
    if (isMini) {
      hideSplash(200);
      return () => {};
    }

    void getCurrentWindow()
      .show()
      .catch(() => {});

    let offConsole: (() => void) | undefined;
    let consoleDisposed = false;
    if (import.meta.env.PROD) {
      void attachConsole().then(
        (off) => {
          if (consoleDisposed) off();
          else offConsole = off;
        },
        () => {},
      );
    }

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
        toasts.success(
          t("vault.downloadComplete", {
            n: summary.done,
            unit: summary.done === 1 ? t("common.trackLc") : t("common.tracks"),
          }),
        );
      }
    });
    window.addEventListener("keydown", handleGlobalKey);
    window.addEventListener("pagehide", flushCloudSyncPush);
    window.addEventListener("online", retryPendingSync);
    const onFullscreenToggle = (): void => {
      showFullscreen = !showFullscreen;
    };
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
      consoleDisposed = true;
      offConsole?.();
      deepLinkDisposed = true;
      offDeepLink?.();
      disposeMini?.();
      cleanup?.();
      offToast();
      setPlayHandler(null);
      window.removeEventListener("keydown", handleGlobalKey);
      window.removeEventListener("pagehide", flushCloudSyncPush);
      window.removeEventListener("online", retryPendingSync);
      document.removeEventListener(
        "vynl:toggle-fullscreen",
        onFullscreenToggle,
      );
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

  function ctxItems(): CtxEntry[] {
    const m = ctxMenu;
    if (!m || m.type !== "playlist" || !m.playlistId || !m.playlistName)
      return [];
    const id = m.playlistId;
    const name = m.playlistName;
    const items: CtxEntry[] = [
      {
        label: t("playlist.editPlaylist"),
        icon: Pencil,
        action: () => {
          editTarget = { id, name, cover: getPlaylistCustomCover(id) ?? null };
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
        onReorderPlaylists={(fromId, toId) =>
          void reorderPlaylists(fromId, toId)}
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
              <CataloguePage
                route={catalogue}
                onNavigate={(p) => navigate(p)}
              />
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

    <PlayerBar
      onOpenPlayer={openPlayer}
      fullscreen={showFullscreen}
      active={page === "player" || showFullscreen}
      onToggleFullscreen={() => {
        showFullscreen = !showFullscreen;
      }}
    />
  </div>

  {#if showFullscreen}
    <FullscreenPlayer
      onclose={() => {
        showFullscreen = false;
      }}
    />
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
      onClose={() => (ctxMenu = null)}
      onNewPlaylist={ctxMenu.type === "rail" ? newPlaylist : undefined}
      onMoveToVynl={ctxMenu.type === "rail"
        ? () => (moveToVynlOpen = true)
        : undefined}
      items={ctxItems()}
    />
  {/if}

  <PlaylistModals bind:createOpen={createTarget} bind:editTarget onnavigate={navigate} />

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

  @container player (max-width: 420px) {
    main {
      padding: 0 10px;
    }

    .page-wrap.bleed {
      margin: 0 -10px;
    }
  }
</style>
