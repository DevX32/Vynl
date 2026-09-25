<script lang="ts">
  import { onMount } from "svelte";
  import { refreshLibrary } from "@state/library.svelte";
  import {
    getCurrentTrack,
    getIsTrackPlaying,
  } from "@state/now-playing.svelte";
  import { clearPlayError, seekOffset } from "@lib/player.svelte";
  import { isInputField } from "@state/shortcuts.svelte";
  import { getCurrentPlayError } from "@state/player.svelte";
  import {
    getLyricsOpen,
    toggleLyrics,
    closeLyrics,
  } from "@state/player.svelte";
  import NowPlayingArt from "./NowPlayingArt.svelte";
  import QueueList from "../queue/QueueList.svelte";
  import LyricsOverlay from "./LyricsOverlay.svelte";
  import ArtBackdrop from "../../components/ArtBackdrop.svelte";

  let {
    active,
  }: {
    active: boolean;
  } = $props();

  let scanning = $state(false);

  const playError = $derived(getCurrentPlayError());

  async function refresh(): Promise<void> {
    scanning = true;
    clearPlayError();
    try {
      await refreshLibrary();
    } catch (e) {
      console.warn("refreshLibrary failed:", e);
    } finally {
      scanning = false;
    }
  }

  $effect(() => {
    if (active) void refresh();
  });

  onMount(() => {
    const onKey = (e: KeyboardEvent): void => {
      if (!active || e.repeat || e.defaultPrevented) return;
      if (isInputField(e.target)) return;
      if (e.key === "Escape" && getLyricsOpen()) {
        closeLyrics();
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        seekOffset(5);
      } else if (e.key === "ArrowLeft") {
        e.preventDefault();
        seekOffset(-5);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
</script>

<section class="player-page">
  {#if getCurrentTrack()?.cover}
    <ArtBackdrop path={getCurrentTrack()!.cover} />
  {/if}
  <div class="stage">
    <div class="main-col">
      <NowPlayingArt
        current={getCurrentTrack()}
        playing={getIsTrackPlaying()}
        {playError}
        {scanning}
        lyricsOpen={getLyricsOpen()}
        onToggleLyrics={toggleLyrics}
      />
      {#if getLyricsOpen()}
        <LyricsOverlay onclose={closeLyrics} />
      {/if}
    </div>
  </div>

  <QueueList />
</section>

<style>
  .player-page {
    height: 100%;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    min-width: 0;
    min-height: 0;
    overflow: clip;
    container-type: inline-size;
    container-name: player;
    position: relative;
  }

  .stage {
    grid-column: 1;
    grid-row: 1;
    min-height: 0;
    min-width: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    position: relative;
    z-index: 2;
  }

  .main-col {
    position: relative;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
</style>
