<script lang="ts">
  import { Music2, X } from "lucide-svelte";
  import { fade } from "svelte/transition";
  import type { NowPlaying } from "../../lib/types";
  import { blobSrc } from "../../lib/format";
  import { t } from "@lib/i18n";

  let {
    current,
    playing,
    playError,
    scanning,
    lyricsOpen,
    onToggleLyrics,
  }: {
    current: NowPlaying | null;
    playing: boolean;
    playError: string | null;
    scanning: boolean;
    lyricsOpen: boolean;
    onToggleLyrics: () => void;
  } = $props();

  const dust = Array.from({ length: 14 }, (_, i) => ({
    id: i,
    x: 4 + ((i * 31) % 90),
    delay: (i * 0.8) % 7,
    dur: 8 + ((i * 5) % 9),
  }));

  const VINYL_DEG_PER_FRAME = 360 / 8.2 / 60;
  const EASE = 0.045;

  let vinylDeg = $state(0);
  let vinylSpeed = 0;
  let raf = 0;

  function loop() {
    const targetVinyl = playing ? VINYL_DEG_PER_FRAME : 0;
    vinylSpeed += (targetVinyl - vinylSpeed) * EASE;
    vinylDeg = (vinylDeg + vinylSpeed) % 360;
    raf = requestAnimationFrame(loop);
  }

  $effect(() => {
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  });
</script>

<div class="now">
  <div class="now-art" class:playing class:empty={!current}>
    <div class="vinyl-slot" aria-hidden="true">
      <div class="vinyl" style:transform={`rotate(${vinylDeg}deg)`}></div>
    </div>
    {#key current?.cover ?? current?.title ?? "none"}
      {#if current?.cover}
        <img
          class="now-cover"
          use:blobSrc={current.cover}
          alt=""
          draggable="false"
          in:fade={{ duration: 260 }}
        />
      {:else}
        <div class="now-cover placeholder" in:fade={{ duration: 260 }}>
          <span class="mono" class:empty={!current}
            >{current ? current.title.charAt(0).toUpperCase() : "·"}</span
          >
        </div>
      {/if}
    {/key}
  </div>

  {#if current}
    {#key current.title}
      <div class="now-title display" in:fade={{ duration: 220 }}>
        {current.title}
      </div>
      <div class="now-artist mono" in:fade={{ duration: 220, delay: 40 }}>
        {current.artist || t("player.unknownArtist")}
      </div>
    {/key}
    <button
      class="lyr-btn"
      class:active={lyricsOpen}
      onclick={onToggleLyrics}
      aria-label={t("player.lyrics")}
    >
      <Music2 size={14} stroke-width={1.5} />
    </button>
    {#if playError}
      <div class="play-error mono" in:fade={{ duration: 180 }}>
        <X size={12} stroke-width={1.5} /> <span>{playError}</span>
      </div>
    {/if}
  {:else if scanning}
    <div class="now-empty mono">{t("player.scanning")}</div>
  {:else}
    <div class="now-title display">{t("player.nothingPlaying")}</div>
  {/if}

  <div class="dust" aria-hidden="true">
    {#each dust as d (d.id)}
      <span
        class="dust-p"
        style:left={`${d.x}%`}
        style:animation-delay={`${d.delay}s`}
        style:animation-duration={`${d.dur}s`}
      ></span>
    {/each}
  </div>
</div>

<style>
  .now {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 26px 0 6px;
    gap: 4px;
  }

  .now-art {
    position: relative;
    width: min(272px, 32vh);
    aspect-ratio: 1 / 1;
    height: auto;
    flex-shrink: 1;
    margin: 18px auto 22px;
  }

  .now-art::before {
    content: "";
    position: absolute;
    inset: -64px;
    border-radius: 50%;
    background: radial-gradient(
      circle,
      color-mix(in srgb, var(--accent) 9%, transparent) 0%,
      transparent 62%
    );
    pointer-events: none;
    z-index: -1;
    transition: opacity 0.6s ease;
  }

  .now-art.playing::before {
    animation: glow-pulse 3.2s ease-in-out infinite;
  }

  @keyframes glow-pulse {
    0%,
    100% {
      opacity: 0.7;
      transform: scale(1);
    }
    50% {
      opacity: 1;
      transform: scale(1.08);
    }
  }

  .vinyl-slot {
    position: absolute;
    inset: 0;
    transform: translateX(14px);
    transition: transform 0.55s cubic-bezier(0.22, 1, 0.36, 1);
    will-change: transform;
  }

  .now-art.playing .vinyl-slot {
    transform: translateX(40px);
  }

  .now-art.empty .vinyl-slot {
    transform: translateX(4px);
  }

  .vinyl {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: #14141c;
    overflow: hidden;
    will-change: transform;
    box-shadow:
      inset 0 0 0 3px rgba(0, 0, 0, 0.45),
      inset 0 0 14px 2px rgba(0, 0, 0, 0.55);
    transition: filter 0.5s ease;
  }

  .vinyl::before {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: repeating-radial-gradient(
      circle at 50% 50%,
      rgba(255, 255, 255, 0.05) 0 1px,
      transparent 1px 6px
    );
    mask-image: radial-gradient(
      circle at 50% 50%,
      transparent 0 21px,
      black 24px
    );
  }

  .vinyl::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background:
      radial-gradient(
        circle at 50% 50%,
        var(--accent) 0 9px,
        color-mix(in srgb, var(--accent) 55%, #14141c) 9px 20px,
        transparent 20px
      ),
      linear-gradient(
        150deg,
        rgba(255, 255, 255, 0.14) 0%,
        transparent 30%,
        transparent 60%,
        rgba(255, 255, 255, 0.05) 100%
      );
    pointer-events: none;
  }

  .now-art.empty .vinyl {
    filter: saturate(0.5) brightness(0.8);
  }

  .now-cover {
    position: absolute;
    z-index: 1;
    left: 50%;
    top: 50%;
    transform: translate(calc(-50% - 26px), -50%);
    width: 71%;
    height: 71%;
    border-radius: var(--radius-sm);
    object-fit: cover;
    background: var(--bg-raise);
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.55);
    transition: box-shadow 0.3s ease;
  }

  .now-cover::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: 8px;
    background: linear-gradient(
      115deg,
      transparent 32%,
      rgba(255, 255, 255, 0.16) 46%,
      transparent 60%
    );
    background-size: 250% 100%;
    background-position: 135% 0;
    opacity: 0;
    transition: opacity 0.4s ease;
    pointer-events: none;
  }

  .now-art.playing .now-cover::after {
    opacity: 1;
    animation: cover-shine 5.5s ease-in-out infinite;
  }

  @keyframes cover-shine {
    0%,
    50% {
      background-position: 135% 0;
    }
    75%,
    100% {
      background-position: -35% 0;
    }
  }

  .now-art.playing .now-cover {
    box-shadow:
      0 18px 50px rgba(0, 0, 0, 0.55),
      0 0 0 1px color-mix(in srgb, var(--accent) 65%, transparent);
  }

  .now-cover.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--placeholder-gradient);
  }

  .now-cover.placeholder span {
    font-size: 46px;
    line-height: 1;
    color: var(--faint);
  }

  .now-cover.placeholder span.empty {
    transform: translateY(-2px);
  }

  .now-title {
    font-size: 30px;
    line-height: 1.12;
    max-width: 640px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .now-artist {
    font-size: 12.5px;
    color: var(--dim);
    margin-top: 4px;
  }

  .play-error {
    margin-top: 10px;
    color: var(--red);
    font-size: 11.5px;
  }

  .lyr-btn {
    margin: 14px auto 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    color: var(--dim);
    background: var(--bg-raise);
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 0;
    cursor: pointer;
    transform: rotate(45deg);
    transition:
      color 0.15s,
      border-color 0.15s,
      background 0.15s;
  }

  .lyr-btn :global(svg) {
    transform: rotate(-45deg);
  }

  .lyr-btn:hover {
    color: var(--dim);
    border-color: var(--line-strong);
    background: var(--bg-raise);
  }

  .lyr-btn.active {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
    background: var(--accent-soft);
  }

  .now-empty {
    color: var(--faint);
    font-size: 11.5px;
  }

  .dust {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
    z-index: 1;
  }

  .dust-p {
    position: absolute;
    bottom: -6px;
    width: 2px;
    height: 2px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--accent) 45%, transparent);
    box-shadow: 0 0 6px 1px color-mix(in srgb, var(--accent) 30%, transparent);
    opacity: 0;
    animation: dust-rise linear infinite;
  }

  @keyframes dust-rise {
    0% {
      transform: translate(0, 0);
      opacity: 0;
    }
    12% {
      opacity: 0.7;
    }
    85% {
      opacity: 0.12;
    }
    100% {
      transform: translate(16px, -380px);
      opacity: 0;
    }
  }
</style>