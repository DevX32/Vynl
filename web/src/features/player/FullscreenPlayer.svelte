<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { Volume2, VolumeOff, X } from "lucide-svelte";
  import { getCurrentTrack, getIsTrackPlaying, getCurrentTime } from "@state/now-playing.svelte";
  import {
    getCurrentMuted,
    getCurrentVolume,
    toggleMute,
    getLyricsOpen,
    toggleLyrics,
    closeLyrics,
  } from "@state/player.svelte";
  import { fmtTime } from "../../lib/format";
  import { t } from "@lib/i18n";
  import NowPlayingArt from "./NowPlayingArt.svelte";
  import LyricsOverlay from "./LyricsOverlay.svelte";
  import ArtBackdrop from "../../components/ArtBackdrop.svelte";
  import SeekSlider from "./SeekSlider.svelte";
  import TransportControls from "./TransportControls.svelte";
  import VolumeSlider from "./VolumeSlider.svelte";

  let { onclose }: { onclose: () => void } = $props();

  let seekDur = $derived(getCurrentTrack()?.duration ?? 0);

  function close(): void {
    if (getLyricsOpen()) {
      closeLyrics();
    } else {
      onclose();
    }
  }

  onMount(() => {
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

</script>

<div class="fs-overlay" transition:fade={{ duration: 200 }}>
  {#if getCurrentTrack()?.cover}
    <ArtBackdrop path={getCurrentTrack()!.cover} showGlow={true} />
  {/if}

  <button class="fs-close" onclick={close} aria-label={t("player.closeFullscreen")}>
    <X size={14} stroke-width={1.5} />
  </button>

  <div class="fs-content">
    {#if !getLyricsOpen()}
      <div class="fs-art">
        <NowPlayingArt
          current={getCurrentTrack()}
          playing={getIsTrackPlaying()}
          playError={null}
          scanning={false}
          lyricsOpen={false}
          onToggleLyrics={toggleLyrics}
        />
      </div>

      <div class="fs-controls">
        <div class="fs-transport">
          <TransportControls />
        </div>

        <div class="fs-seek">
          <span class="time mono">{fmtTime(getCurrentTime())}</span>
          <SeekSlider />
          <span class="time mono">{fmtTime(seekDur)}</span>
        </div>

        <div class="fs-vol">
          <button class="vol-btn" onclick={() => { toggleMute(); }} aria-label={getCurrentMuted() ? t("player.unmute") : t("player.mute")}>
            {#if getCurrentMuted() || getCurrentVolume() === 0}
              <VolumeOff size={18} stroke-width={1.5} />
            {:else}
              <Volume2 size={18} stroke-width={1.5} />
            {/if}
          </button>
          <VolumeSlider />
          <span class="vol-pct mono">{Math.round(getCurrentVolume() * 100)}%</span>
        </div>
      </div>
    {/if}
  </div>

  {#if getLyricsOpen()}
    <div class="fs-lyrics">
      <LyricsOverlay onclose={closeLyrics} fullscreen={true} />
    </div>
  {/if}
</div>

<style>
  .fs-overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg);
  }

  .fs-content {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    padding: 32px;
    width: 100%;
    max-width: 600px;
    height: 100vh;
  }

  .fs-close {
    position: fixed;
    right: 10px;
    top: 50%;
    transform: translateY(-50%);
    z-index: 210;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line);
    color: var(--text);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transform: rotate(45deg);
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .fs-close :global(svg) {
    transform: rotate(-45deg);
  }

  .fs-close:hover,
  .fs-overlay:hover .fs-close {
    opacity: 1;
    background: color-mix(in srgb, var(--bg-raise) 80%, var(--accent-soft));
    border-color: var(--line-strong);
  }

  .fs-art {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 0 120px;
  }

  .fs-art :global(.now-art) {
    width: min(320px, 42vh) !important;
    aspect-ratio: 1 / 1 !important;
  }

  .fs-art :global(.dust) {
    position: fixed;
    inset: 0;
    z-index: 2;
  }

  .fs-art :global(.ring) {
    inset: -52px !important;
    width: calc(100% + 104px) !important;
    height: calc(100% + 104px) !important;
  }

  .fs-controls {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    width: 100%;
    max-width: 500px;
  }

  .fs-transport {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 24px;
    --tc-icon-color: var(--dim);
    --tc-disabled-opacity: 0.3;
  }

  .fs-seek {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
  }

  .fs-seek .time {
    font-size: 11px;
    color: var(--faint);
    min-width: 38px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }

  .fs-vol {
    display: flex;
    align-items: center;
    gap: 10px;
    --vol-width: 120px;
  }

  .fs-vol .vol-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    background: none;
    border: none;
    color: var(--faint);
    cursor: pointer;
    transform: rotate(45deg);
    transition: color 0.15s;
  }

  .fs-vol .vol-btn :global(svg) {
    transform: rotate(-45deg);
  }

  .fs-vol .vol-btn:hover {
    color: var(--text);
  }

  .fs-vol .vol-pct {
    font-size: 11px;
    color: var(--faint);
    width: 32px;
    text-align: right;
  }

  .fs-lyrics {
    position: fixed;
    inset: 0;
    z-index: 5;
    width: 100%;
    height: 100%;
  }

  .fs-lyrics :global(.lyrics-overlay) {
    padding: 0;
  }

  .fs-lyrics :global(.lyrics-page) {
    height: 100%;
    padding: 16px 24px;
  }

</style>
