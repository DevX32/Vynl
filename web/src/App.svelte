<script lang="ts">
  import { onMount } from "svelte";
  import { setupApp } from "@lib/app-lifecycle";
  import { nav, navigate, goBack, goForward, setSidebarOpen } from "@lib/app-nav.svelte";
  import { vynl } from "./lib/vynl";
  import {
    getCurrentTrack,
    getIsTrackPlaying,
  } from "@state/now-playing.svelte";
  import { miniTrackSnapshot, sendMiniState } from "@lib/mini-sync";
  import MiniPlayer from "./features/player/MiniPlayer.svelte";
  import { getCurrentSettings } from "@state/settings.svelte";
  import { t } from "@lib/i18n";
  import { toasts } from "./lib/toast";
  import { toggleShortcutsOverlay } from "@state/shortcuts.svelte";
  import Onboarding from "./features/onboarding/Onboarding.svelte";
  import {
    getCurrentPlaylists,
    getSelectedId,
    refreshPlaylists,
    selectPlaylist,
    reorderPlaylists,
    setCurrentPlaylist,
  } from "@state/playlists.svelte";
  import { showCtx, showRailCtx } from "@lib/app-ctx.svelte";
  import type { StreamImportResult } from "./lib/types";
  import AppOverlays from "./features/shell/AppOverlays.svelte";
  import AppShell from "./features/shell/AppShell.svelte";

  const isMini =
    typeof window !== "undefined" && window.location.hash.startsWith("#mini");

  let showFullscreen = $state(false);
  let onboarded = $state(true);
  let accountOpen = $state(false);


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

  onMount(() => setupApp({
    isMini,
    setOnboarded: (v) => (onboarded = v),
    onToggleFullscreen: () => (showFullscreen = !showFullscreen),
  }));

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

</script>

{#if isMini}
  <MiniPlayer />
{:else if !onboarded}
  <Onboarding onComplete={() => (onboarded = true)} />
{:else}
  <AppShell
    page={nav.page}
    historyIdx={nav.historyIdx}
    historyLength={nav.history.length}
    sidebarOpen={nav.sidebarOpen}
    {playlists}
    selectedId={getSelectedId()}
    {showFullscreen}
    onBack={goBack}
    onForward={goForward}
    onHome={openHome}
    onToggleSidebar={() => setSidebarOpen(!nav.sidebarOpen)}
    onShowShortcuts={toggleShortcutsOverlay}
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
    onNavigate={navigate}
    onOpenVault={openVault}
    onOpenPlayer={openPlayer}
    onToggleFullscreen={() => (showFullscreen = !showFullscreen)}
  />

  <AppOverlays
    bind:showFullscreen
    bind:accountOpen
    bind:moveToVynlOpen
    bind:createTarget
    bind:editTarget
    onNavigate={navigate}
    {onMovedToVynl}
    onNewPlaylist={newPlaylist}
  />
{/if}

