<script lang="ts">
  import { RotateCcw } from "lucide-svelte";
  import { t } from "@lib/i18n";
  import { getCurrentSettings, patchSettings } from "@state/settings.svelte";
  import { clamp01 } from "@lib/pointer";
  import { EQ_PRESETS, type EqPreset } from "@lib/constants";
  import {
    eqBands,
    eqFlat,
    eqMaxDb,
    eqMinDb,
    eqResidualBoostDb,
  } from "@lib/eq.svelte";
  import EqCurve from "./EqCurve.svelte";
  import Button from "@components/Button.svelte";
  import Switch from "@components/Switch.svelte";

  const settings = $derived(getCurrentSettings());

  const TRACK_H = 128;
  const THUMB = 9;
  const RANGE = (): number => eqMaxDb() - eqMinDb();

  const GROUP_RANGES: { key: string; from: number; to: number }[] = [
    { key: "eq.groups.sub", from: 31, to: 62 },
    { key: "eq.groups.low", from: 125, to: 250 },
    { key: "eq.groups.mid", from: 500, to: 2000 },
    { key: "eq.groups.high", from: 4000, to: 8000 },
    { key: "eq.groups.air", from: 16000, to: 16000 },
  ];

  const GROUP_LABEL: (string | undefined)[] = $derived.by(() => {
    const labels: (string | undefined)[] = [];
    let seen = "";
    for (const freq of eqBands()) {
      const g = GROUP_RANGES.find((r) => freq >= r.from && freq <= r.to);
      if (!g || g.key === seen) {
        labels.push(undefined);
        continue;
      }
      seen = g.key;
      labels.push(g.key);
    }
    return labels;
  });

  let bands = $state<number[]>(eqFlat());
  let touched = $state(false);

  $effect(() => {
    if (touched) return;
    const stored = settings.eqBands ?? [];
    const next = eqBands().map((_, i) =>
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
    const count = eqBands().length;
    const gains = preset.gains.slice(0, count);
    while (gains.length < count) gains.push(0);
    bands = gains;
    scheduleBands();
  }

  function resetAll(): void {
    touched = true;
    bands = eqFlat();
    scheduleBands();
  }

  const activePreset = $derived(
    EQ_PRESETS.find(
      (p) =>
        p.gains.length === bands.length &&
        p.gains.every((g, i) => g === bands[i]),
    )?.id ?? null,
  );

  const designReady = $derived(eqBands().length > 0);
  const isModified = $derived(bands.some((g) => g !== 0));
  const isFlatCurve = $derived(bands.every((g) => Math.abs(g) < 0.05));
  const clipRisk = $derived(eqResidualBoostDb(bands) > 6);

  function ratio(i: number): number {
    return (bands[i] - eqMinDb()) / RANGE();
  }

  function fill(i: number): number {
    const r = ratio(i);
    return ((THUMB / 2 + r * (TRACK_H - THUMB)) / TRACK_H) * 100;
  }

  function freqLabel(freq: number): string {
    return freq >= 1000 ? `${freq / 1000}k` : String(freq);
  }

  function dbLabel(db: number): string {
    return db > 0 ? `+${db}` : String(db);
  }

  function setBand(i: number, db: number): void {
    db = Math.max(eqMinDb(), Math.min(eqMaxDb(), Math.round(db)));
    if (db === bands[i]) return;
    touched = true;
    bands[i] = db;
    scheduleBands();
  }

  function bandFromPointer(e: PointerEvent, i: number): void {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const r = 1 - clamp01((e.clientY - rect.top) / rect.height);
    setBand(i, eqMinDb() + r * RANGE());
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
      case "Delete":
      case "Backspace":
        db = 0;
        break;
      default:
        return;
    }
    e.preventDefault();
    setBand(i, db);
  }
</script>

<div class="section">
  <div class="head">
    <div class="head-text">
      <div class="hint mono">
        {settings.eqEnabled
          ? t("settings.equalizerHint")
          : t("settings.equalizerHintOff")}
      </div>
    </div>
    <div class="head-actions">
      <button
        class="reset"
        class:ready={isModified}
        onclick={resetAll}
        disabled={!settings.eqEnabled || !isModified}
        aria-label={t("eq.reset")}
        title={t("eq.reset")}
      >
        <RotateCcw size={13} stroke-width={1.6} />
      </button>
      <Switch
        on={settings.eqEnabled}
        label={t("settings.equalizerToggle")}
        onclick={() => setEnabled(!settings.eqEnabled)}
      />
    </div>
  </div>

  {#if !designReady}
    <div class="unavailable mono">{t("eq.unavailable")}</div>
  {:else}
    <div class="eq" class:off={!settings.eqEnabled}>
    <div class="chips">
      {#each EQ_PRESETS as preset (preset.id)}
        <Button
          size="sm"
          selected={activePreset === preset.id}
          disabled={!settings.eqEnabled}
          onclick={() => applyPreset(preset)}
        >
          {t(preset.labelKey)}
        </Button>
      {/each}
    </div>

    <div class="curve-wrap">
      <EqCurve {bands} disabled={!settings.eqEnabled} />
      {#if isFlatCurve}
        <div class="curve-note mono">{t("eq.curveFlat")}</div>
      {/if}
      {#if clipRisk}
        <div class="curve-warn mono">{t("eq.clipRisk")}</div>
      {/if}
    </div>

    <div class="bands">
      {#each eqBands() as freq, i (freq)}
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
            aria-valuemin={eqMinDb()}
            aria-valuemax={eqMaxDb()}
            aria-valuenow={bands[i]}
            aria-valuetext={`${dbLabel(bands[i])} dB`}
            aria-disabled={!settings.eqEnabled}
            onpointerdown={(e) => bandDown(e, i)}
            onpointermove={(e) => bandMove(e, i)}
            ondblclick={() => setBand(i, 0)}
            onkeydown={(e) => bandKey(e, i)}
          >
            <span
              class="thumb"
              style:bottom={`calc((100% - ${THUMB}px) * ${ratio(i)})`}
            ></span>
          </div>
          <div class="freq mono">{freqLabel(freq)}</div>
          <div class="group mono">
            {#if GROUP_LABEL[i]}{t(GROUP_LABEL[i]!)}{:else}&nbsp;{/if}
          </div>
        </div>
      {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .unavailable {
    font-size: 11px;
    color: var(--faint);
    padding: 24px 0;
    text-align: center;
    line-height: 1.5;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 14px;
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

  .head-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .hint {
    font-size: 11px;
    color: var(--faint);
    line-height: 1.5;
  }

  .reset {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    color: var(--faint);
    opacity: 0;
    transition:
      opacity 0.15s,
      color 0.15s,
      background 0.15s;
  }

  .reset.ready {
    opacity: 1;
  }

  .reset:hover:not(:disabled) {
    color: var(--text);
    background: var(--bg-raise);
  }

  .reset:disabled {
    cursor: default;
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

  .curve-wrap {
    position: relative;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    background: var(--bg);
    padding: 2px 0;
  }

  .curve-note,
  .curve-warn {
    position: absolute;
    left: 0;
    right: 0;
    text-align: center;
    pointer-events: none;
    font-size: 9.5px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .curve-note {
    top: 50%;
    transform: translateY(-50%);
    color: var(--faint);
    opacity: 0.7;
  }

  .curve-warn {
    bottom: 3px;
    color: var(--amber);
    opacity: 0.85;
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

  .group {
    font-size: 8.5px;
    color: var(--faint);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    opacity: 0.7;
    margin-top: -4px;
  }
</style>