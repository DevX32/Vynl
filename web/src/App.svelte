<script lang="ts">
  import { onMount } from "svelte";
  import { vynl } from "./lib/vynl";
  import { initSettings, getCurrentSettings } from "@state/settings.svelte";
  import { t } from "@lib/i18n";
  import { refreshLibrary } from "@state/library.svelte";
  import { getPluginsUiOpen, initPlugins } from "@state/plugins.svelte";
  import { emitAppEvent } from "@lib/plugins/events";
  import { restoreNowPlaying, setPlayHandler } from "@state/now-playing.svelte";
  import { playTrack } from "@lib/player.svelte";
  import { handleGlobalKey, getShowShortcutsOverlay } from "@state/shortcuts.svelte";
  import { getEqOpen } from "@state/equalizer.svelte";
  import { toasts } from "./lib/toast";
  import { toPascalCase } from "./lib/format";
  import {
    getCurrentPlaylists,
    getSelectedId,
    refreshPlaylists,
    selectPlaylist,
    renamePlaylist,
    deletePlaylist,
    createPlaylist,
  } from "@state/playlists.svelte";
  import ContextMenu from "./components/ContextMenu.svelte";
  import Dialog from "./components/Dialog.svelte";
  import Toast from "./components/Toast.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import PlayerPage from "./features/player/PlayerPage.svelte";
  import PlayerBar from "./features/player/PlayerBar.svelte";
  import FullscreenPlayer from "./features/player/FullscreenPlayer.svelte";
  import LibraryPage from "./features/playlists/LibraryPage.svelte";
  import PlaylistDetail from "./features/playlists/PlaylistDetail.svelte";
  import SettingsPage from "./features/settings/SettingsPage.svelte";
  import EqualizerModal from "./features/settings/EqualizerModal.svelte";
  import PluginsModal from "./features/plugins/PluginsModal.svelte";
  import VaultPage from "./features/vault/VaultPage.svelte";
  import HomePage from "./features/home/HomePage.svelte";
  import PluginPageHost from "./features/plugins/PluginPageHost.svelte";
  import ArtistPage from "./features/artist/ArtistPage.svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import ShortcutsOverlay from "./components/ShortcutsOverlay.svelte";
  import { isPluginPage, type Page } from "@lib/types";

  let page = $state<Page>("home");
  let sidebarOpen = $state(true);
  let showFullscreen = $state(false);
  let selectedArtist = $state<string | null>(null);

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

  const playlists = $derived(getCurrentPlaylists());

  function setSidebarOpen(open: boolean): void {
    sidebarOpen = open;
  }

  const SPLASH_MIN_MS = 2500;
  const splashReady = Date.now();

  function hideSplash(): void {
    const elapsed = Date.now() - splashReady;
    const wait = Math.max(0, SPLASH_MIN_MS - elapsed);
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
    void vynl.rpcUpdate(null);
    await refreshPlaylists();
    await refreshLibrary();
    await restoreNowPlaying();
    void initPlugins().catch(console.warn);
    hideSplash();
    const off = vynl.onDownloadEvent((e) => {
      if (e.type === "finished") {
        void Promise.all([refreshPlaylists(), refreshLibrary()]).finally(() => {
          emitAppEvent("download-finished");
        });
      }
    });
    const offUpdate = vynl.onUpdateStatus((s) => {
      if (s.available && !s.downloading && !s.ready && !s.error) {
        toasts.success(t("update.availableTitle"));
      }
      if (s.ready) {
        toasts.success(t("update.readyTitle"));
      }
      if (s.error) {
        toasts.error(t("update.failedTitle"));
      }
    });
    void vynl.checkAppUpdate().catch(() => {
      toasts.error(t("update.failedTitle"));
    });
    return () => { off(); offUpdate(); };
  }

  onMount(() => {
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
    let disposed = false;
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

  function openVault(): void {
    navigate("vault");
  }

  function openArtist(name: string): void {
    selectedArtist = name;
  }

  function closeArtist(): void {
    selectedArtist = null;
  }

  function openLibrary(): void {
    navigate("library");
    void selectPlaylist(null);
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
      await deletePlaylist(target.id);
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

  function handleCtxMenuAction(): void {
    const m = ctxMenu;
    if (!m || m.type !== "playlist" || !m.playlistId || !m.playlistName) return;
    deleteTarget = { id: m.playlistId, name: m.playlistName };
  }

  function handleCtxRename(): void {
    const m = ctxMenu;
    if (!m || m.type !== "playlist" || !m.playlistId || !m.playlistName) return;
    renameTarget = { id: m.playlistId, name: m.playlistName };
  }
</script>

<div class="shell">
  <TitleBar
    onBack={goBack}
    onForward={goForward}
    canBack={historyIdx > 0}
    canForward={historyIdx < history.length - 1}
  />
  <div class="body">
    <Sidebar
      {page}
      {sidebarOpen}
      {playlists}
      selectedId={getSelectedId()}
      onSetSidebarOpen={setSidebarOpen}
      onHome={openHome}
      onLibrary={openLibrary}
      onPlayer={openPlayer}
      onVault={openVault}
      onSettings={openSettings}
      onPluginPage={(id) => navigate(`plugin:${id}`)}
      onNewPlaylist={newPlaylist}
      onOpenPlaylist={(id) => void openPlaylist(id)}
      onShowCtx={showCtx}
      onShowRailCtx={showRailCtx}
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
        {:else if page === "playlist"}
          <div class="page-inner">
            <PlaylistDetail />
          </div>
        {:else if page === "player"}
          <div class="page-inner">
            <PlayerPage active={true} />
          </div>
        {:else if page === "settings"}
          <div class="page-inner">
            <SettingsPage />
          </div>
        {:else if isPluginPage(page)}
          <PluginPageHost pageId={page.slice("plugin:".length)} />
        {/if}
        <div class="page-inner" class:hidden={page !== "vault"}>
          <VaultPage active={page === "vault"} />
        </div>
      </div>
    </main>
  </div>

  <PlayerBar onOpenPlayer={openPlayer} fullscreen={showFullscreen} onToggleFullscreen={() => { showFullscreen = !showFullscreen; }} onArtistClick={openArtist} />
</div>

{#if selectedArtist}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div class="artist-overlay" role="dialog" tabindex="-1" onclick={(e) => { if (e.target === e.currentTarget) closeArtist(); }} onkeydown={(e) => { if (e.key === "Escape") closeArtist(); }}>
    <div class="artist-modal">
      <ArtistPage artist={selectedArtist} onBack={closeArtist} />
    </div>
  </div>
{/if}

{#if showFullscreen}
  <FullscreenPlayer onclose={() => { showFullscreen = false; }} />
{/if}

{#if getShowShortcutsOverlay()}
  <ShortcutsOverlay />
{/if}

{#if getEqOpen()}
  <EqualizerModal />
{/if}

{#if getPluginsUiOpen()}
  <PluginsModal />
{/if}

{#if ctxMenu}
  <ContextMenu
    x={ctxMenu.x}
    y={ctxMenu.y}
    onclose={() => (ctxMenu = null)}
    onrename={ctxMenu.type === "playlist" ? handleCtxRename : undefined}
    ondelete={ctxMenu.type === "playlist" ? handleCtxMenuAction : undefined}
    onnewplaylist={ctxMenu.type === "rail" ? newPlaylist : undefined}
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

  .artist-overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(6px);
  }

  .artist-modal {
    width: min(600px, 85vw);
    max-height: 78vh;
    background: var(--bg);
    border: 1px solid var(--line);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    position: relative;
  }

  .artist-modal :global(.artist-page) {
    height: auto;
    min-height: 0;
  }

  .artist-modal :global(.view) {
    max-height: 78vh;
    overflow-y: auto;
    padding: 40px 44px 48px;
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