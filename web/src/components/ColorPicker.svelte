<script lang="ts">
  import { untrack } from "svelte";
  import { Palette, Check } from "lucide-svelte";
  import {
    isValidHexColor,
    normalizeHex,
    hexToHsv,
    hsvToHex,
  } from "@lib/color";
  import { clamp01, pointerRatio } from "@lib/pointer";

  let {
    value,
    presets,
    onchange,
    label = "Custom color",
    disabled = false,
  }: {
    value: string;
    presets: readonly string[];
    onchange: (hex: string) => void;
    label?: string;
    disabled?: boolean;
  } = $props();

  let rootEl: HTMLDivElement | undefined = $state();
  let svEl: HTMLDivElement | undefined = $state();
  let hueEl: HTMLDivElement | undefined = $state();

  let open = $state(false);
  let hue = $state(0);
  let sat = $state(0);
  let val = $state(0);
  let hexText = $state("");

  function sameColor(a: string, b: string): boolean {
    return a.toLowerCase() === b.toLowerCase();
  }

  function syncFromValue(): void {
    const hsv = hexToHsv(value);
    if (hsv) {
      hue = hsv.h;
      sat = hsv.s;
      val = hsv.v;
    }
    hexText = normalizeHex(value).slice(1);
  }

  $effect(() => {
    if (open) {
      untrack(() => syncFromValue());
    }
  });

  function applyHsv(): void {
    const hex = hsvToHex(hue, sat, val);
    hexText = hex.slice(1);
    onchange(hex);
  }

  function svPointer(e: PointerEvent): void {
    if (!svEl) return;
    const rect = svEl.getBoundingClientRect();
    sat = clamp01((e.clientX - rect.left) / rect.width) * 100;
    val = (1 - clamp01((e.clientY - rect.top) / rect.height)) * 100;
    applyHsv();
  }

  function handleSvDown(e: PointerEvent): void {
    if (disabled) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    svPointer(e);
  }

  function handleSvMove(e: PointerEvent): void {
    if (e.buttons !== 1) return;
    svPointer(e);
  }

  function huePointer(e: PointerEvent): void {
    if (!hueEl) return;
    hue = pointerRatio(hueEl, e.clientX) * 360;
    applyHsv();
  }

  const HUE_THUMB_SIZE = 14;
  const SV_THUMB_SIZE = 12;
  function hueThumbLeft(): string {
    const t = clamp01(hue / 360);
    return `calc((100% - ${HUE_THUMB_SIZE}px) * ${t})`;
  }

  function handleHueDown(e: PointerEvent): void {
    if (disabled) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    huePointer(e);
  }

  function handleHueMove(e: PointerEvent): void {
    if (e.buttons !== 1) return;
    huePointer(e);
  }

  function commitHexText(): void {
    if (disabled) return;
    const candidate = hexText.trim().startsWith("#")
      ? hexText.trim()
      : `#${hexText.trim()}`;
    if (isValidHexColor(candidate)) {
      const hex = normalizeHex(candidate);
      const hsv = hexToHsv(hex);
      if (hsv) {
        hue = hsv.h;
        sat = hsv.s;
        val = hsv.v;
      }
      hexText = hex.slice(1);
      onchange(hex);
    } else {
      hexText = normalizeHex(value).slice(1);
    }
  }

  function handleHexKey(e: KeyboardEvent): void {
    if (disabled) return;
    if (e.key === "Enter") {
      e.preventDefault();
      commitHexText();
      (e.currentTarget as HTMLInputElement).blur();
    } else if (e.key === "Escape") {
      e.preventDefault();
      hexText = normalizeHex(value).slice(1);
      (e.currentTarget as HTMLInputElement).blur();
    }
  }

  function commitPreset(color: string): void {
    if (disabled) return;
    onchange(normalizeHex(color));
    open = false;
  }

  function togglePopover(): void {
    if (disabled) {
      open = false;
      return;
    }
    open = !open;
  }

  function handleClickOutside(e: MouseEvent): void {
    if (open && rootEl && !rootEl.contains(e.target as Node)) open = false;
  }

  function handleWindowKey(e: KeyboardEvent): void {
    if (open && e.key === "Escape") open = false;
  }
</script>

<svelte:window onclick={handleClickOutside} onkeydown={handleWindowKey} />

<div class="color-picker" class:disabled bind:this={rootEl}>
  <div class="swatches">
    {#each presets as color}
      {@const selected = sameColor(value, color)}
      <button
        type="button"
        class="swatch"
        class:selected={selected}
        style:background={color}
        onclick={() => commitPreset(color)}
        disabled={disabled}
        aria-label={color}
        aria-pressed={selected}
      >
        {#if selected}
          <span class="swatch-check">
            <Check size={11} stroke-width={2.5} />
          </span>
        {/if}
      </button>
    {/each}
    <button
      type="button"
      class="swatch custom"
      class:selected={!presets.some((c) => sameColor(value, c))}
      style:background={value}
      disabled={disabled}
      aria-label={label}
      aria-haspopup="dialog"
      aria-expanded={open}
      onclick={togglePopover}
    >
      {#if !presets.some((c) => sameColor(value, c))}
        <span class="swatch-check">
          <Check size={11} stroke-width={2.5} />
        </span>
      {:else}
        <Palette size={12} stroke-width={1.5} />
      {/if}
    </button>
  </div>

  {#if open}
    <div class="popover" role="dialog" aria-label={label}>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="sv"
        bind:this={svEl}
        onpointerdown={handleSvDown}
        onpointermove={handleSvMove}
      >
        <div class="sv-surface" style:background={`hsl(${hue}, 100%, 50%)`}>
          <div class="sv-white"></div>
          <div class="sv-black"></div>
        </div>
        <div
          class="sv-thumb"
          style:left={`calc((100% - ${SV_THUMB_SIZE}px) * ${sat / 100})`}
          style:top={`calc((100% - ${SV_THUMB_SIZE}px) * ${(100 - val) / 100})`}
          style:background={hsvToHex(hue, sat, val)}
        ></div>
      </div>

      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="hue"
        bind:this={hueEl}
        onpointerdown={handleHueDown}
        onpointermove={handleHueMove}
      >
        <div
          class="hue-indicator"
          style:left={hueThumbLeft()}
          style:background={`hsl(${hue}, 100%, 50%)`}
        ></div>
      </div>

      <div class="hex-row">
        <span class="hex-prefix mono">#</span>
        <input
          class="hex-input mono"
          value={hexText}
          maxlength="6"
          spellcheck={false}
          oninput={(e) => (hexText = e.currentTarget.value)}
          onkeydown={handleHexKey}
          onblur={commitHexText}
        />
      </div>
    </div>
  {/if}
</div>

<style>
  .color-picker {
    position: relative;
    display: inline-flex;
  }

  .color-picker.disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
  }

  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 4px;
    margin: -4px;
  }

  .swatch {
    position: relative;
    width: 26px;
    height: 26px;
    border-radius: var(--radius-pill);
    border: 2px solid transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition:
      border-color 0.28s ease,
      box-shadow 0.28s ease,
      transform 0.28s cubic-bezier(0.25, 0.8, 0.25, 1);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.06);
    will-change: transform, box-shadow;
    transform: translateZ(0);
  }

  .swatch:hover {
    transform: translateZ(0) scale(1.08);
  }

  .swatch.selected {
    border-color: var(--bg);
    box-shadow:
      0 0 0 2px var(--accent),
      0 2px 10px rgba(0, 0, 0, 0.35),
      inset 0 0 0 1px rgba(255, 255, 255, 0.06);
    transform: translateZ(0) scale(1.04);
  }

  .swatch.custom {
    color: var(--accent-text);
  }

  .swatch-check {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: rgba(14, 14, 16, 0.78);
    color: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(2px);
    animation: check-in 0.28s cubic-bezier(0.25, 0.8, 0.25, 1);
    will-change: transform, opacity;
  }

  @keyframes check-in {
    from {
      opacity: 0;
      transform: scale(0.55);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .popover {
    position: absolute;
    z-index: 100;
    top: calc(100% + 10px);
    left: -4px;
    width: 192px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    box-shadow: var(--shadow-lg);
    animation: picker-in 0.2s cubic-bezier(0.25, 0.8, 0.25, 1);
    will-change: transform, opacity;
  }

  @keyframes picker-in {
    from {
      opacity: 0;
      transform: translateY(-6px) scale(0.97);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  .sv {
    position: relative;
    width: 100%;
    height: 120px;
    cursor: pointer;
    touch-action: none;
    clip-path: inset(0 round var(--radius-sm));
    -webkit-transform: translateZ(0);
    transform: translateZ(0);
    -webkit-backface-visibility: hidden;
    backface-visibility: hidden;
  }

  .sv-surface {
    position: absolute;
    inset: 0;
    -webkit-transform: translateZ(0);
    transform: translateZ(0);
  }

  .sv-white {
    position: absolute;
    inset: 0;
    background: linear-gradient(to right, #fff, rgba(255, 255, 255, 0));
  }

  .sv-black {
    position: absolute;
    inset: 0;
    background: linear-gradient(to top, #000, rgba(0, 0, 0, 0));
  }

  .sv-thumb {
    position: absolute;
    box-sizing: border-box;
    width: 12px;
    height: 12px;
    border-radius: 2px;
    border: 2px solid #fff;
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.4),
      0 1px 3px rgba(0, 0, 0, 0.3);
    pointer-events: none;
  }

  .hue {
    position: relative;
    width: 100%;
    height: 18px;
    border-radius: 2px;
    cursor: pointer;
    touch-action: none;
    background: linear-gradient(
      to right,
      hsl(0, 100%, 50%),
      hsl(30, 100%, 50%),
      hsl(60, 100%, 50%),
      hsl(90, 100%, 50%),
      hsl(120, 100%, 50%),
      hsl(150, 100%, 50%),
      hsl(180, 100%, 50%),
      hsl(210, 100%, 50%),
      hsl(240, 100%, 50%),
      hsl(270, 100%, 50%),
      hsl(300, 100%, 50%),
      hsl(330, 100%, 50%),
      hsl(360, 100%, 50%)
    );
  }

  .hue-indicator {
    position: absolute;
    box-sizing: border-box;
    top: 50%;
    width: 14px;
    height: 14px;
    border-radius: 2px;
    border: 2px solid #fff;
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.4),
      0 1px 3px rgba(0, 0, 0, 0.3);
    transform: translateY(-50%);
    pointer-events: none;
  }

  .hex-row {
    display: flex;
    align-items: center;
    gap: 4px;
    background: var(--bg-raise);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 6px 10px;
  }

  .hex-prefix {
    font-size: 12px;
    color: var(--faint);
  }

  .hex-input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    outline: none;
    color: var(--text);
    font-size: 12px;
    text-transform: lowercase;
  }
</style>