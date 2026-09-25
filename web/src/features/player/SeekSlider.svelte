<script lang="ts">
  import { onDestroy } from "svelte";
  import {
    getCurrentTime,
    getCurrentTrack,
    setSeekDragging,
  } from "@state/now-playing.svelte";
  import { cancelSeek, seekFlush, seekOffset, seekRatio } from "@lib/player.svelte";
  import { pointerRatio } from "@lib/pointer";
  import { t } from "@lib/i18n";

  let seekDur = $derived(getCurrentTrack()?.duration ?? 0);

  let el: HTMLDivElement = $state()!;
  let dragging = $state(false);

  let pct = $derived(
    seekDur > 0 ? Math.min((getCurrentTime() / seekDur) * 100, 100) : 0,
  );

  function onSeek(e: PointerEvent): void {
    if (!el) return;
    seekRatio(pointerRatio(el, e.clientX), seekDur);
  }

  function endSeekDrag(e?: PointerEvent): void {
    if (!dragging) return;
    seekFlush();
    setSeekDragging(false);
    dragging = false;
    if (e && el?.hasPointerCapture(e.pointerId)) {
      el.releasePointerCapture(e.pointerId);
    }
  }

  function onKey(e: KeyboardEvent): void {
    if (!getCurrentTrack()) return;
    if (e.key === "ArrowRight" || e.key === "ArrowLeft") {
      e.preventDefault();
      e.stopPropagation();
      seekOffset(e.key === "ArrowRight" ? seekDur / 20 : -seekDur / 20);
    }
  }

  // The slider is being torn down mid-drag (fullscreen closed): drop the
  // pending seek instead of committing it.
  onDestroy(() => {
    if (!dragging) return;
    cancelSeek();
    setSeekDragging(false);
    dragging = false;
  });
</script>

<div
  class="seek"
  class:active={dragging}
  bind:this={el}
  onpointerdown={(e) => {
    if (!getCurrentTrack()) return;
    dragging = true;
    setSeekDragging(true);
    el.setPointerCapture(e.pointerId);
    onSeek(e);
  }}
  onpointermove={(e) => {
    if (dragging) onSeek(e);
  }}
  onpointerup={endSeekDrag}
  onpointercancel={endSeekDrag}
  onlostpointercapture={() => endSeekDrag()}
  role="slider"
  aria-label={t("player.seek")}
  aria-valuemin="0"
  aria-valuemax={seekDur}
  aria-valuenow={Math.round(getCurrentTime())}
  aria-disabled={!getCurrentTrack()}
  tabindex="0"
  onkeydown={onKey}
>
  <div class="seek-fill" style:width={`${pct}%`}></div>
  <div class="seek-thumb" style:left={`${pct}%`}></div>
</div>

<style>
  .seek {
    flex: 1;
    height: 16px;
    position: relative;
    cursor: pointer;
    display: flex;
    align-items: center;
    border-radius: 2px;
    transition: opacity 0.15s;
    touch-action: none;
  }

  .seek::before {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    height: 3px;
    border-radius: var(--radius-sm);
    background: var(--line-strong);
  }

  .seek-fill {
    position: absolute;
    left: 0;
    top: 50%;
    height: 3px;
    border-radius: var(--radius-sm);
    background: var(--accent);
    pointer-events: none;
    transform: translateY(-50%);
  }

  .seek-thumb {
    position: absolute;
    top: 50%;
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: var(--accent);
    transform: translate(-50%, -50%) rotate(45deg);
    pointer-events: none;
    opacity: 0;
    transition:
      opacity 0.18s ease-out,
      box-shadow 0.18s ease-out;
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 20%, transparent);
  }

  .seek:hover .seek-thumb,
  .seek.active .seek-thumb {
    opacity: 1;
  }
</style>
