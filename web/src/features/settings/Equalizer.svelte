<script lang="ts">
  import { t } from "@lib/i18n";
  import { getCurrentSettings, patchSettings } from "@state/settings.svelte";
  import { clamp01 } from "@lib/pointer";
  import {
    EQ_BANDS,
    EQ_FLAT,
    EQ_MAX_DB,
    EQ_MIN_DB,
    EQ_PRESETS,
    type EqPreset,
  } from "@lib/constants";

  let {
    framed = true,
    showLabel = true,
  }: {
    framed?: boolean;
    showLabel?: boolean;
  } = $props();

  const settings = $derived(getCurrentSettings());

  const TRACK_H = 128;
  const THUMB = 9;
  const RANGE = EQ_MAX_DB - EQ_MIN_DB;

  let bands = $state<number[]>([...EQ_FLAT]);
  let touched = $state(false);

  $effect(() => {
    if (touched) return;
    const stored = settings.eqBands ?? [];
    const next = EQ_BANDS.map((_, i) =>
      Number.isFinite(stored[i]) ? stored[i] : 0,
    );
    if (next.some((v, i) => v !== bands[i])) bands = next;
  });

  let patchTimer: ReturnType<typeof setTimeout> | null = null;
  function scheduleBands(): void {
    if (patchTimer) clearTimeout(patchTimer);
    patchTimer = setTimeout(() => {
      patchTimer = null;
      void patchSettings({ eqBands: [...bands] });
    }, 160);
  }

  function setEnabled(on: boolean): void {
    void patchSettings({ eqEnabled: on });
  }

  function applyPreset(preset: EqPreset): void {
    touched = true;
    bands = [...preset.gains];
    scheduleBands();
  }

  const activePreset = $derived(
    EQ_PRESETS.find((p) => p.gains.every((g, i) => g === bands[i]))?.id ?? null,
  );

  function ratio(i: number): number {
    return (bands[i] - EQ_MIN_DB) / RANGE;
  }

  function fill(i: number): number {
    const t = ratio(i);
    return ((THUMB / 2 + t * (TRACK_H - THUMB)) / TRACK_H) * 100;
  }

  function freqLabel(freq: number): string {
    return freq >= 1000 ? `${freq / 1000}k` : String(freq);
  }

  function dbLabel(db: number): string {
    return db > 0 ? `+${db}` : String(db);
  }

  function setBand(i: number, db: number): void {
    db = Math.max(EQ_MIN_DB, Math.min(EQ_MAX_DB, Math.round(db)));
    if (db === bands[i]) return;
    touched = true;
    bands[i] = db;
    scheduleBands();
  }

  function bandFromPointer(e: PointerEvent, i: number): void {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const t = 1 - clamp01((e.clientY - rect.top) / rect.height);
    setBand(i, EQ_MIN_DB + t * RANGE);
  }

  function bandDown(e: PointerEvent, i: number): void {
    if (!settings.eqEnabled) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    bandFromPointer(e, i);
  }

  function bandMove(e: PointerEvent, i: number): void {
    if (e.buttons !== 1) return;
    bandFromPointer(e, i);
  }

  function bandKey(e: KeyboardEvent, i: number): void {
    if (!settings.eqEnabled) return;
    let db = bands[i];
    switch (e.key) {
      case "ArrowUp":
      case "ArrowRight":
        db += 1;
        break;
      case "ArrowDown":
      case "ArrowLeft":
        db -= 1;
        break;
      case "PageUp":
        db += 3;
        break;
      case "PageDown":
        db -= 3;
        break;
      case "Home":
        db = 0;
        break;
      default:
        return;
    }
    e.preventDefault();
    setBand(i, db);
  }
</script>

<div class="section" class:framed>
  <div class="head">
    <div class="head-text">
      {#if showLabel}
        <div class="label mono">{t("settings.equalizer")}</div>
      {/if}
      <div class="hint mono">
        {settings.eqEnabled
          ? t("settings.equalizerHint")
          : t("settings.equalizerHintOff")}
      </div>
    </div>
    <button
      class="switch"
      class:on={settings.eqEnabled}
      onclick={() => setEnabled(!settings.eqEnabled)}
      role="switch"
      aria-checked={settings.eqEnabled}
      aria-label={t("settings.equalizerToggle")}
    >
      <span class="knob"></span>
    </button>
  </div>

  <div class="eq" class:off={!settings.eqEnabled}>
    <div class="chips">
      {#each EQ_PRESETS as preset (preset.id)}
        <button
          class="chip mono"
          class:selected={activePreset === preset.id}
          disabled={!settings.eqEnabled}
          onclick={() => applyPreset(preset)}
        >
          {preset.label}
        </button>
      {/each}
    </div>

    <div class="bands">
      {#each EQ_BANDS as freq, i (freq)}
        <div class="band">
          <div class="db mono" class:boost={bands[i] > 0} class:cut={bands[i] < 0}>
            {dbLabel(bands[i])}
          </div>
          <div
            class="track"
            style:--fill={`${fill(i)}%`}
            role="slider"
            tabindex={settings.eqEnabled ? 0 : -1}
            aria-label={`${freqLabel(freq)} ${t("settings.equalizerBand")}`}
            aria-valuemin={EQ_MIN_DB}
            aria-valuemax={EQ_MAX_DB}
            aria-valuenow={bands[i]}
            aria-valuetext={`${dbLabel(bands[i])} dB`}
            aria-disabled={!settings.eqEnabled}
            onpointerdown={(e) => bandDown(e, i)}
            onpointermove={(e) => bandMove(e, i)}
            onkeydown={(e) => bandKey(e, i)}
          >
            <span
              class="thumb"
              style:bottom={`calc((100% - ${THUMB}px) * ${ratio(i)})`}
            ></span>
          </div>
          <div class="freq mono">{freqLabel(freq)}</div>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .section {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .section.framed {
    padding: 16px;
    border-top: 1px solid var(--line);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .head-text {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .label {
    font-size: 11px;
    color: var(--dim);
    letter-spacing: 0.1em;
  }

  .hint {
    font-size: 11px;
    color: var(--faint);
    line-height: 1.5;
  }

  .eq {
    display: flex;
    flex-direction: column;
    gap: 14px;
    transition: opacity 0.2s;
  }

  .eq.off {
    opacity: 0.45;
    pointer-events: none;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    font-size: 11px;
    color: var(--dim);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 4px 10px;
    transition:
      color 0.15s,
      background 0.15s;
  }

  .chip:hover:not(:disabled) {
    color: var(--text);
    background: var(--bg-raise);
  }

  .chip.selected {
    color: var(--accent);
    background: var(--accent-soft);
  }

  .chip:disabled {
    cursor: not-allowed;
  }

  .bands {
    display: flex;
    gap: 4px;
    overflow-x: auto;
    scrollbar-width: none;
    padding-bottom: 2px;
  }

  .bands::-webkit-scrollbar {
    display: none;
  }

  .band {
    flex: 1;
    min-width: 34px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  .db {
    font-size: 10.5px;
    color: var(--faint);
    height: 13px;
    transition: color 0.15s;
  }

  .db.boost {
    color: var(--accent);
  }

  .db.cut {
    color: var(--dim);
  }

  .track {
    position: relative;
    width: 24px;
    height: 128px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    touch-action: none;
    user-select: none;
    outline: none;
    transition: box-shadow 0.15s;
  }

  .track::before {
    content: "";
    position: absolute;
    left: 50%;
    top: 0;
    bottom: 0;
    width: 3px;
    margin-left: -1.5px;
    border-radius: var(--radius-sm);
    background: var(--line-strong);
  }

  .track::after {
    content: "";
    position: absolute;
    left: 50%;
    bottom: 0;
    width: 3px;
    height: var(--fill, 50%);
    margin-left: -1.5px;
    border-radius: var(--radius-sm);
    background: var(--accent);
    pointer-events: none;
  }

  .track:focus-visible {
    box-shadow: 0 0 0 2px var(--accent-soft), 0 0 0 1px var(--accent);
  }

  .thumb {
    position: absolute;
    left: 50%;
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 20%, transparent);
    transform: translateX(-50%) rotate(45deg);
    pointer-events: none;
    z-index: 1;
    transition: box-shadow 0.18s ease-out;
  }

  .freq {
    font-size: 10px;
    color: var(--faint);
    letter-spacing: 0.02em;
  }
</style>
