<script lang="ts">
  import { getCurrentVolume } from "@state/player.svelte";
  import { setVolumeAt } from "@lib/player.svelte";
  import { pointerRatio } from "@lib/pointer";
  import { t } from "@lib/i18n";

  let el: HTMLDivElement = $state()!;
  let dragging = $state(false);

  function onVol(e: PointerEvent): void {
    if (!el) return;
    setVolumeAt(pointerRatio(el, e.clientX));
  }

  function endVolDrag(e?: PointerEvent): void {
    dragging = false;
    if (e && el?.hasPointerCapture(e.pointerId)) {
      el.releasePointerCapture(e.pointerId);
    }
  }

  function onKey(e: KeyboardEvent): void {
    if (e.key === "ArrowRight" || e.key === "ArrowUp") {
      e.preventDefault();
      e.stopPropagation();
      setVolumeAt(getCurrentVolume() + 0.05);
    }
    if (e.key === "ArrowLeft" || e.key === "ArrowDown") {
      e.preventDefault();
      e.stopPropagation();
      setVolumeAt(getCurrentVolume() - 0.05);
    }
  }
</script>

<div
  class="vol-line"
  class:active={dragging}
  bind:this={el}
  role="slider"
  aria-label={t("player.volume")}
  aria-valuemin="0"
  aria-valuemax="1"
  aria-valuenow={getCurrentVolume()}
  tabindex="0"
  onpointerdown={(e) => {
    dragging = true;
    el.setPointerCapture(e.pointerId);
    onVol(e);
  }}
  onpointermove={(e) => {
    if (dragging) onVol(e);
  }}
  onpointerup={endVolDrag}
  onpointercancel={endVolDrag}
  onlostpointercapture={() => endVolDrag()}
  onkeydown={onKey}
>
  <div class="vol-track"></div>
  <div class="vol-fill" style:width={`${getCurrentVolume() * 100}%`}></div>
  <div class="vol-thumb" style:left={`${getCurrentVolume() * 100}%`}></div>
</div>

<style>
  .vol-line {
    position: relative;
    width: var(--vol-width, 120px);
    height: 14px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    cursor: pointer;
    touch-action: none;
  }

  .vol-track {
    position: absolute;
    left: 0;
    right: 0;
    height: 3px;
    border-radius: var(--radius-sm);
    background: var(--line-strong);
    pointer-events: none;
  }

  .vol-fill {
    position: absolute;
    left: 0;
    top: 50%;
    height: 3px;
    border-radius: var(--radius-sm);
    background: var(--accent);
    pointer-events: none;
    transform: translateY(-50%);
  }

  .vol-thumb {
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

  .vol-line:hover .vol-thumb,
  .vol-line.active .vol-thumb {
    opacity: 1;
  }
</style>
