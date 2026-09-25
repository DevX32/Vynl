<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { Volume2, VolumeOff, Expand, Shrink, SlidersHorizontal } from "lucide-svelte";
  import { getCurrentTrack, getCurrentTime } from "@state/now-playing.svelte";
  import { getCurrentMuted, getCurrentVolume, toggleMute } from "@state/player.svelte";
  import { applyVolume } from "@lib/player.svelte";
  import { blobSrc, fmtTime } from "../../lib/format";
  import { t } from "@lib/i18n";
  import SeekSlider from "./SeekSlider.svelte";
  import TransportControls from "./TransportControls.svelte";
  import VolumeSlider from "./VolumeSlider.svelte";
  import { getCurrentSettings } from "@state/settings.svelte";
  import { getEqOpen, openEq } from "@state/equalizer.svelte";

  let {
    onOpenPlayer,
    fullscreen = false,
    onToggleFullscreen,
    onArtistClick,
  }: {
    onOpenPlayer: () => void;
    fullscreen?: boolean;
    onToggleFullscreen: () => void;
    onArtistClick?: (artist: string) => void;
  } = $props();

  let seekDur = $derived(getCurrentTrack()?.duration ?? 0);

  function toggleFullscreen(): void {
    onToggleFullscreen();
  }

  onMount(() => {
    applyVolume();
  });
</script>

<div class="player-bar" class:empty={!getCurrentTrack()}>
  <button
    class="now-mini"
    onclick={onOpenPlayer}
    aria-label={t("player.openPlayer")}
  >
    <div class="mini-cover-wrap">
      {#if getCurrentTrack()?.cover}
        {#key getCurrentTrack()!.cover}
          <img
            class="mini-cover"
            use:blobSrc={getCurrentTrack()!.cover}
            alt=""
            draggable="false"
            transition:fade={{ duration: 300 }}
          />
        {/key}
      {:else}
        <div class="mini-cover placeholder" class:empty={!getCurrentTrack()}>
          {getCurrentTrack()
            ? getCurrentTrack()!.title.charAt(0).toUpperCase()
            : "·"}
        </div>
      {/if}
    </div>
    <div class="mini-meta">
      <span class="mini-title display">
        {getCurrentTrack()?.title ?? t("player.nothingPlaying")}</span
      >
      <span class="mini-artist mono">
        {#if getCurrentTrack()?.artist}
          {#each getCurrentTrack()!.artist.split(", ") as artistName, i}
            {#if i > 0}
              <span class="artist-sep">, </span>
            {/if}
            <span
              class="artist-link"
              role="button"
              tabindex="0"
              onclick={(e) => {
                e.stopPropagation();
                onArtistClick?.(artistName);
              }}
              onkeydown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.stopPropagation();
                  onArtistClick?.(artistName);
                }
              }}
            >{artistName}</span>
          {/each}
        {:else}
          <span>{t("player.unknownArtist")}</span>
        {/if}
      </span>
    </div>
  </button>

  <div class="center-stack">
    <div class="transport">
      <TransportControls />
    </div>

    <div class="seek-group">
      <span class="time mono">{fmtTime(getCurrentTime())}</span>
      <SeekSlider />
      <span class="time mono">{fmtTime(seekDur)}</span>
    </div>
  </div>

  <div class="bar-right">
    <div class="vol">
      <button
        class="vol-btn"
        onclick={() => {
          toggleMute();
        }}
        aria-label={getCurrentMuted() ? t("player.unmute") : t("player.mute")}
      >
        {#if getCurrentMuted() || getCurrentVolume() === 0}
          <VolumeOff size={14} stroke-width={1.5} />
        {:else}
          <Volume2 size={14} stroke-width={1.5} />
        {/if}
      </button>
      <VolumeSlider />
      <span class="vol-pct mono">{Math.round(getCurrentVolume() * 100)}%</span>
    </div>
    <button
      class="icon-btn small"
      class:on={getCurrentSettings().eqEnabled}
      onclick={openEq}
      aria-label={t("player.equalizer")}
      aria-pressed={getEqOpen()}
    >
      <SlidersHorizontal size={12} stroke-width={1.5} />
    </button>
    <span class="bar-sep"></span>
    <button
      class="icon-btn small"
      class:on={fullscreen}
      onclick={toggleFullscreen}
      aria-label={t("player.fullscreen")}
      aria-pressed={fullscreen}
    >
      {#if fullscreen}
        <Shrink size={12} stroke-width={1.5} />
      {:else}
        <Expand size={12} stroke-width={1.5} />
      {/if}
    </button>
  </div>
</div>

<style>
  .player-bar {
    grid-column: 1 / -1;
    grid-row: 2;
    flex-shrink: 0;
    width: 100%;
    min-width: 0;
    margin: 0;
    padding: 18px 32px;
    background: linear-gradient(
      180deg,
      var(--bg),
      color-mix(in srgb, var(--bg) 94%, var(--accent) 2%)
    );
    border-top: 1px solid var(--line);
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.8fr) minmax(0, 1fr);
    align-items: center;
    gap: clamp(8px, 2cqi, 24px);
  }

  /* Transport buttons live in TransportControls; pass the idle dim down as a
     custom property so it doesn't stack on the component's own disabled dim. */
  .player-bar.empty .transport {
    --tc-idle-opacity: 0.4;
  }

  .player-bar.empty .vol,
  .player-bar.empty .seek-group,
  .player-bar.empty .bar-right .icon-btn {
    opacity: 0.4;
  }

  .now-mini {
    width: 100%;
    max-width: 340px;
    min-width: 0;
    justify-self: start;
    display: flex;
    align-items: center;
    gap: 12px;
    background: none;
    border: none;
    padding: 0;
    text-align: left;
    cursor: pointer;
    border-radius: var(--radius-sm);
  }

  .mini-cover-wrap {
    position: relative;
    width: 58px;
    height: 58px;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    box-shadow: var(--shadow-sm);
  }

  .mini-cover {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border-radius: var(--radius-sm);
    object-fit: cover;
    background: var(--bg-raise);
  }

  .mini-cover.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--placeholder-gradient);
    font-family: var(--font-mono);
    color: var(--faint);
    font-size: 14px;
    line-height: 1;
  }

  .mini-cover.placeholder.empty {
    transform: translateY(-1px);
  }

  .mini-meta {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .mini-title {
    font-size: 15px;
    font-weight: 600;
    line-height: 1.2;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: color 0.15s;
  }

  .mini-artist {
    font-size: 11.5px;
    letter-spacing: 0.04em;
    color: var(--dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: flex;
    align-items: center;
    gap: 0;
  }

  .artist-link {
    background: none;
    border: none;
    padding: 0;
    color: var(--dim);
    font: inherit;
    font-size: inherit;
    letter-spacing: inherit;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: color 0.15s;
  }

  .artist-sep {
    color: var(--dim);
    font: inherit;
    font-size: inherit;
    letter-spacing: inherit;
  }

  .artist-link:hover {
    color: var(--text);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .center-stack {
    width: 100%;
    max-width: 480px;
    justify-self: center;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .seek-group {
    width: 100%;
    min-width: 0;
    margin-top: 8px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .time {
    font-size: 10.5px;
    color: var(--faint);
    min-width: 28px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }

  .transport {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: clamp(6px, 2.4cqi, 18px);
  }

  .bar-right {
    justify-self: end;
    min-width: 0;
    max-width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .bar-right .icon-btn {
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    background: none;
    border-color: transparent;
    transform: rotate(45deg);
    transition:
      transform 0.15s,
      color 0.15s,
      background 0.15s,
      border-color 0.15s,
      box-shadow 0.15s;
  }

  .bar-right .icon-btn :global(svg) {
    transform: rotate(-45deg);
  }

  .bar-right .icon-btn:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--line-strong);
    background: var(--bg-raise);
    box-shadow: var(--shadow-sm);
  }

  .bar-right .icon-btn.on {
    color: var(--accent);
    background: var(--accent-soft);
  }

  .vol {
    width: auto;
    min-width: 0;
    flex-shrink: 1;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 7px;
    color: var(--faint);
    --vol-width: 76px;
  }

  .bar-sep {
    width: 1px;
    height: 16px;
    background: var(--line);
    margin: 0 3px;
  }

  .vol-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    background: none;
    border: none;
    padding: 0;
    color: var(--faint);
    cursor: pointer;
    transition: color 0.15s;
  }

  .vol-btn:hover {
    color: var(--text);
  }

  .vol-pct {
    font-size: 11px;
    color: var(--faint);
    width: 30px;
    text-align: right;
    flex-shrink: 0;
  }

  @container player (max-width: 780px) {
    .player-bar {
      padding: 14px 16px;
    }

    .now-mini {
      max-width: 280px;
    }

    .mini-cover-wrap {
      width: 48px;
      height: 48px;
    }

    .vol {
      --vol-width: 48px;
      gap: 5px;
    }

    .vol-pct {
      display: none;
    }

    .bar-right {
      gap: 5px;
    }
  }

  @container player (max-width: 620px) {
    .player-bar {
      grid-template-columns: minmax(0, 1fr) auto;
      grid-template-rows: auto auto;
      row-gap: 10px;
    }

    .now-mini {
      grid-column: 1;
      grid-row: 1;
      max-width: none;
    }

    .bar-right {
      grid-column: 2;
      grid-row: 1;
    }

    .center-stack {
      grid-column: 1 / -1;
      grid-row: 2;
      max-width: none;
    }

    .vol {
      --vol-width: 44px;
    }
  }

  @container player (max-width: 420px) {
    .mini-cover-wrap {
      width: 40px;
      height: 40px;
    }

    .mini-artist {
      display: none;
    }

    .vol {
      display: none;
    }

    .bar-sep {
      display: none;
    }

    .transport :global(.icon-btn.small) {
      display: none;
    }

    .transport {
      gap: 10px;
    }
  }
</style>
