<script lang="ts">
  import type { CataloguePageRoute, Page } from "@lib/types";
  import CatalogueSearchPage from "./CatalogueSearchPage.svelte";
  import CatalogueAlbumPage from "./CatalogueAlbumPage.svelte";
  import CatalogueArtistPage from "./CatalogueArtistPage.svelte";
  import CataloguePlaylistPage from "./CataloguePlaylistPage.svelte";

  let {
    route,
    onNavigate,
  }: {
    route: CataloguePageRoute;
    onNavigate: (page: Page) => void;
  } = $props();
</script>

{#if route.view === "search"}
  <CatalogueSearchPage {onNavigate} />
{:else if route.view === "album"}
  {#key route.id}
    <CatalogueAlbumPage id={route.id} {onNavigate} />
  {/key}
{:else if route.view === "artist"}
  {#key route.id}
    <CatalogueArtistPage id={route.id} {onNavigate} />
  {/key}
{:else if route.view === "playlist"}
  {#key route.id}
    <CataloguePlaylistPage id={route.id} {onNavigate} />
  {/key}
{/if}
