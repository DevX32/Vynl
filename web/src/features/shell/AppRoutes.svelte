<script lang="ts">
  import HomePage from "../home/HomePage.svelte";
  import LibraryPage from "../playlists/LibraryPage.svelte";
  import CataloguePage from "../catalogue/CataloguePage.svelte";
  import PlaylistDetail from "../playlists/PlaylistDetail.svelte";
  import PlayerPage from "../player/PlayerPage.svelte";
  import SettingsPage from "../settings/SettingsPage.svelte";
  import VaultPage from "../vault/VaultPage.svelte";
  import { catalogueRoute, type Page } from "@lib/types";

  let {
    page,
    showFullscreen,
    onNavigate,
    onOpenVault,
  }: {
    page: Page;
    showFullscreen: boolean;
    onNavigate: (p: Page) => void;
    onOpenVault: () => void;
  } = $props();

  const catalogue = $derived(catalogueRoute(page));
</script>

<div class="page-wrap" class:bleed={page === "player"}>
  {#if page === "home"}
    <div class="page-inner">
      <HomePage onNavigate={onNavigate} />
    </div>
  {:else if page === "library"}
    <div class="page-inner">
      <LibraryPage active={true} />
    </div>
  {:else if catalogue}
    <div class="page-inner">
      <CataloguePage route={catalogue} onNavigate={onNavigate} />
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
      <SettingsPage onOpenVault={onOpenVault} />
    </div>
  {/if}
  <div class="page-inner" class:hidden={page !== "vault"}>
    <VaultPage active={page === "vault"} />
  </div>
</div>

<style>
  .page-wrap { height: 100%; position: relative; }
  .page-wrap.bleed { margin: 0 -32px; }
  .page-inner { height: 100%; }
  .page-inner.hidden { display: none; }
  @container player (max-width: 620px) { .page-wrap.bleed { margin: 0 -16px; } }
  @container player (max-width: 420px) { .page-wrap.bleed { margin: 0 -10px; } }
</style>
