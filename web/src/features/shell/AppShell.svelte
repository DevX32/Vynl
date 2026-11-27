<script lang="ts">
  import TitleBar from "../../components/TitleBar.svelte";
  import Sidebar from "../../components/Sidebar.svelte";
  import PlayerBar from "../player/PlayerBar.svelte";
  import QueueList from "../queue/QueueList.svelte";
  import AppRoutes from "./AppRoutes.svelte";
  import { getQueueOpen } from "@state/queue.svelte";
  import type { Page } from "@lib/types";
  import type { PlaylistMeta } from "@lib/types";

  let {
    page,
    historyIdx,
    historyLength,
    sidebarOpen,
    playlists,
    selectedId,
    showFullscreen,
    onBack,
    onForward,
    onHome,
    onToggleSidebar,
    onShowShortcuts,
    onLibrary,
    onCatalogue,
    onPlayer,
    onVault,
    onSettings,
    onAccount,
    onNewPlaylist,
    onMoveToVynl,
    onOpenPlaylist,
    onOpenLikedSongs,
    onShowCtx,
    onShowRailCtx,
    onReorderPlaylists,
    onNavigate,
    onOpenVault,
    onOpenPlayer,
    onToggleFullscreen,
  }: {
    page: Page;
    historyIdx: number;
    historyLength: number;
    sidebarOpen: boolean;
    playlists: PlaylistMeta[];
    selectedId: string | null;
    showFullscreen: boolean;
    onBack: () => void;
    onForward: () => void;
    onHome: () => void;
    onToggleSidebar: () => void;
    onShowShortcuts: () => void;
    onLibrary: () => void;
    onCatalogue: () => void;
    onPlayer: () => void;
    onVault: () => void;
    onSettings: () => void;
    onAccount: () => void;
    onNewPlaylist: () => void;
    onMoveToVynl: () => void;
    onOpenPlaylist: (id: string) => void;
    onOpenLikedSongs: () => void;
    onShowCtx: (e: MouseEvent, id: string, name: string) => void;
    onShowRailCtx: (e: MouseEvent) => void;
    onReorderPlaylists: (fromId: string, toId: string) => void;
    onNavigate: (p: Page) => void;
    onOpenVault: () => void;
    onOpenPlayer: () => void;
    onToggleFullscreen: () => void;
  } = $props();
</script>

<div class="shell">
  <TitleBar
    {onBack}
    {onForward}
    canBack={historyIdx > 0}
    canForward={historyIdx < historyLength - 1}
    {onHome}
    {sidebarOpen}
    {onToggleSidebar}
    {onShowShortcuts}
  />
  <div class="body">
    <Sidebar
      {page}
      {sidebarOpen}
      {playlists}
      {selectedId}
      {onLibrary}
      {onCatalogue}
      {onPlayer}
      {onVault}
      {onSettings}
      {onAccount}
      {onNewPlaylist}
      {onMoveToVynl}
      {onOpenPlaylist}
      {onOpenLikedSongs}
      {onShowCtx}
      {onShowRailCtx}
      {onReorderPlaylists}
    />

    <main>
      <AppRoutes {page} {showFullscreen} {onNavigate} {onOpenVault} />
    </main>

    {#if getQueueOpen()}
      <QueueList />
    {/if}
  </div>

  <PlayerBar
    {onOpenPlayer}
    fullscreen={showFullscreen}
    active={page === "player" || showFullscreen}
    {onToggleFullscreen}
  />
</div>

<style>
  .shell { height: 100vh; display: flex; flex-direction: column; container-type: inline-size; container-name: player; }
  .body { flex: 1; min-height: 0; display: flex; position: relative; }
  main { flex: 1; min-height: 0; min-width: 0; padding: 0 32px; }
  @container player (max-width: 620px) { main { padding: 0 16px; } }
  @container player (max-width: 420px) { main { padding: 0 10px; } }
</style>
