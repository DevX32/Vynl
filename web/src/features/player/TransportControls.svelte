<script lang="ts">
  import {
    Pause,
    Play,
    Repeat,
    Repeat1,
    Shuffle,
    SkipBack,
    SkipForward,
  } from "lucide-svelte";
  import { getCurrentTrack, getIsTrackPlaying } from "@state/now-playing.svelte";
  import {
    getCurrentLoop,
    getCurrentShuffle,
    toggleLoop,
    toggleShuffle,
  } from "@state/player.svelte";
  import { next, prev, toggle } from "@lib/player.svelte";
  import { t } from "@lib/i18n";
</script>

<button
  class="icon-btn small"
  class:on={getCurrentShuffle()}
  onclick={toggleShuffle}
  aria-label={t("player.shuffle")}
  aria-pressed={getCurrentShuffle()}
>
  <Shuffle size={12} stroke-width={1.5} />
</button>
<button
  class="icon-btn"
  onclick={prev}
  aria-label={t("player.previous")}
  disabled={!getCurrentTrack()}
>
  <SkipBack size={14} fill="currentColor" stroke-width={0} />
</button>
<button
  class="play-btn"
  onclick={toggle}
  aria-label={getIsTrackPlaying() ? t("player.pause") : t("player.play")}
  disabled={!getCurrentTrack()}
>
  <span class="p-icon" class:show={getIsTrackPlaying()}>
    <Pause size={14} fill="currentColor" stroke-width={0} />
  </span>
  <span class="p-icon" class:show={!getIsTrackPlaying()}>
    <Play size={14} fill="currentColor" stroke-width={0} />
  </span>
</button>
<button
  class="icon-btn"
  onclick={next}
  aria-label={t("player.next")}
  disabled={!getCurrentTrack()}
>
  <SkipForward size={14} fill="currentColor" stroke-width={0} />
</button>
<button
  class="icon-btn small"
  class:on={getCurrentLoop() !== "off"}
  onclick={toggleLoop}
  aria-label={getCurrentLoop() === "one"
    ? t("player.repeatOne")
    : getCurrentLoop() === "all"
      ? t("player.repeatAll")
      : t("player.repeatOff")}
  aria-pressed={getCurrentLoop() !== "off"}
>
  {#if getCurrentLoop() === "one"}
    <Repeat1 size={12} stroke-width={1.5} />
  {:else}
    <Repeat size={12} stroke-width={1.5} />
  {/if}
</button>

<style>
  /* Visual knobs supplied by the host bar (PlayerBar / FullscreenPlayer):
     --tc-icon-color, --tc-idle-opacity, --tc-disabled-opacity. */
  .icon-btn {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line);
    color: var(--tc-icon-color, var(--faint));
    opacity: var(--tc-idle-opacity, 1);
    transform: rotate(45deg);
    transition:
      transform 0.15s,
      color 0.15s,
      background 0.15s,
      border-color 0.15s,
      box-shadow 0.15s;
  }

  .icon-btn :global(svg) {
    transform: rotate(-45deg);
  }

  .icon-btn:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--line-strong);
    background: var(--bg-raise);
    box-shadow: var(--shadow-sm);
  }

  .icon-btn.small {
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    background: none;
    border-color: transparent;
  }

  .icon-btn.on {
    color: var(--accent);
    background: var(--accent-soft);
  }

  .icon-btn:disabled {
    opacity: var(--tc-disabled-opacity, var(--tc-idle-opacity, 1));
    cursor: default;
  }

  .play-btn {
    position: relative;
    width: 40px;
    height: 40px;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--accent-text);
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: var(--tc-idle-opacity, 1);
    transform: rotate(45deg);
    transition:
      background 0.15s,
      transform 0.1s,
      box-shadow 0.15s;
  }

  .play-btn .p-icon {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 0.18s ease-out;
  }

  .play-btn .p-icon.show {
    opacity: 1;
  }

  .play-btn :global(svg) {
    transform: rotate(-45deg);
  }

  .play-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 82%, #ffffff);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .play-btn:active:not(:disabled) {
    transform: rotate(45deg) scale(0.92);
  }

  .play-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
