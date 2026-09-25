<script lang="ts">
  import { getCurrentTrack } from "@state/now-playing.svelte";
  import LyricsPage from "../lyrics/LyricsPage.svelte";
  import ArtBackdrop from "../../components/ArtBackdrop.svelte";
  import { fly } from "svelte/transition";
  import { cubicOut, cubicIn } from "svelte/easing";

  let { onclose, fullscreen = false }: { onclose: () => void; fullscreen?: boolean } = $props();
</script>

<div
  class="lyrics-overlay"
  in:fly={{ y: 14, duration: 250, easing: cubicOut }}
  out:fly={{ y: 14, duration: 200, easing: cubicIn }}
>
  {#if getCurrentTrack()?.cover}
    <ArtBackdrop path={getCurrentTrack()!.cover} showGlow={true} />
  {/if}
  <LyricsPage {onclose} {fullscreen} />
</div>

<style>
  .lyrics-overlay {
    position: absolute;
    inset: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    padding: 16px 36px 0;
    overflow: hidden;
  }
</style>
