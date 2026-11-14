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
    lyricsOpen = false,
    onToggleLyrics = () => {},
    showLyricsButton = true,
  }: {
    current: NowPlaying | null;
    playing: boolean;
    playError: string | null;
    scanning: boolean;
    lyricsOpen?: boolean;
    onToggleLyrics?: () => void;
    showLyricsButton?: boolean;
  } = $props();

  const VYNL_RPM = 33 + 1 / 3;
  const VYNL_DEG_PER_SEC = (360 / 60) * VYNL_RPM;
  const EASE = 0.03;
  const SPEED_EPSILON = 0.5;

  let vynlDeg = $state(0);
  let vynlSpeed = 0;
  let raf = 0;
  let lastTs = 0;

  function loop(target: number, ts: number): void {
    const dt = lastTs ? Math.min((ts - lastTs) / 1000, 0.25) : 0;
    lastTs = ts;

    if (dt > 0) {
      const blend = 1 - Math.pow(1 - EASE, dt * 60);
      vynlSpeed += (target - vynlSpeed) * blend;
      vynlDeg = (vynlDeg + vynlSpeed * dt) % 360;
    }

    if (target === 0 && Math.abs(vynlSpeed) < SPEED_EPSILON) {
      vynlSpeed = 0;
      lastTs = 0;
      raf = 0;
      return;
    }
    raf = requestAnimationFrame((next) => loop(target, next));
  }

  $effect(() => {
    const target = playing ? VYNL_DEG_PER_SEC : 0;
    lastTs = 0;
    raf = requestAnimationFrame((ts) => loop(target, ts));
    return () => {
      cancelAnimationFrame(raf);
      raf = 0;
      lastTs = 0;
    };
  });
</script>

<div class="now">
  <div class="now-art" class:playing class:empty={!current}>
    <div class="vynl-slot" aria-hidden="true">
      <div class="vynl" style:transform={`rotate(${vynlDeg}deg)`}></div>
    </div>
    {#key current?.id ?? "none"}
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
          {#if current}
            <span class="ph-initial display"
              >{current.title.charAt(0).toUpperCase()}</span
            >
          {:else}
            <span class="ph">
              <Music2 size={17} stroke-width={1.1} />
              <span class="ph-mark display">VYNL</span>
            </span>
          {/if}
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
    {#if showLyricsButton}
      <button
        class="lyr-btn"
        class:active={lyricsOpen}
        onclick={onToggleLyrics}
        aria-label={t("player.lyrics")}
      >
        <Music2 size={14} stroke-width={1.5} />
      </button>
    {/if}
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
    width: var(--vynl-disc);
    aspect-ratio: 1 / 1;
    height: auto;
    flex-shrink: 1;
    margin: 18px auto 22px;
  }

  .vynl-slot {
    position: absolute;
    inset: 0;
    transform: translateX(3.5%);
    transition:
      transform 0.55s cubic-bezier(0.22, 1, 0.36, 1),
      filter 0.5s ease;
    will-change: transform;
  }

  .now-art.playing .vynl-slot {
    transform: translateX(14.5%);
  }

  .now-art.empty .vynl-slot {
    transform: translateX(1.5%);
    filter: saturate(0.35) brightness(0.72);
  }

  .vynl {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: var(--vynl);
    overflow: hidden;
    will-change: transform;
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.06),
      inset 0 0 0 4px rgba(0, 0, 0, 0.5),
      inset 0 0 20px 3px rgba(0, 0, 0, 0.6);
  }

  .vynl::before {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background:
      repeating-radial-gradient(
        circle at 50% 50%,
        rgba(255, 255, 255, 0.05) 0 0.5%,
        transparent 0.5% 2.9%
      ),
      repeating-radial-gradient(
        circle at 50% 50%,
        rgba(255, 255, 255, 0.028) 0 0.34%,
        transparent 0.34% 1.63%
      );
    mask-image: radial-gradient(
      circle at 50% 50%,
      transparent 0 12%,
      black 13.5%
    );
    -webkit-mask-image: radial-gradient(
      circle at 50% 50%,
      transparent 0 12%,
      black 13.5%
    );
  }

  .vynl-slot::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background:
      conic-gradient(
        from 200deg at 50% 50%,
        transparent 0deg,
        rgba(255, 255, 255, 0.07) 24deg,
        transparent 58deg,
        transparent 188deg,
        rgba(255, 255, 255, 0.05) 214deg,
        transparent 250deg
      ),
      linear-gradient(
        150deg,
        rgba(255, 255, 255, 0.1) 0%,
        transparent 32%
      );
    mix-blend-mode: screen;
    pointer-events: none;
  }

  .now-cover {
    position: absolute;
    z-index: 1;
    left: 50%;
    top: 50%;
    transform: translate(calc(-50% - var(--vynl-disc) * 0.055), -50%);
    width: 72%;
    height: 72%;
    border-radius: var(--radius-sm);
    object-fit: cover;
    background: var(--bg-raise);
    box-shadow: 0 0 0 1px var(--line);
    transition:
      transform 0.55s cubic-bezier(0.22, 1, 0.36, 1),
      box-shadow 0.3s ease;
  }

  .now-art:not(.playing) .now-cover {
    transform: translate(calc(-50% - var(--vynl-disc) * 0.121), -50%);
  }

  .now-art.playing .now-cover {
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 65%, transparent);
  }

  .now-cover.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background:
      linear-gradient(
        155deg,
        rgba(255, 255, 255, 0.05) 0%,
        transparent 55%
      ),
      var(--placeholder-gradient);
    color: var(--faint);
  }

  .ph {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    opacity: 0.85;
  }

  .ph-mark {
    font-size: 8.5px;
    letter-spacing: 0.34em;
    text-indent: 0.34em;
  }

  .ph-initial {
    font-size: 56px;
    line-height: 1;
    opacity: 0.7;
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
    margin: 21px auto 7px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    flex-shrink: 0;
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
</style>
