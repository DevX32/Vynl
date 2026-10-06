<script lang="ts">
  import { t } from "@lib/i18n";
  import { fmtDuration } from "@lib/format";
  import {
    CROSSFADE_STEPS,
    getCrossfade,
    setCrossfade,
  } from "@state/player.svelte";

  const current = $derived(getCrossfade());

  const options = $derived(
    CROSSFADE_STEPS.map((secs) => ({
      secs,
      label: secs === 0 ? t("player.crossfadeOff") : fmtDuration(secs),
    })),
  );

  function choose(secs: number): void {
    setCrossfade(secs);
  }
</script>

<div class="section">
  <div class="hint mono">{t("player.crossfadeHint")}</div>
  <div class="chips">
    {#each options as opt (opt.secs)}
      <button
        class="chip mono"
        class:selected={opt.secs === current}
        aria-pressed={opt.secs === current}
        onclick={() => choose(opt.secs)}
      >
        {opt.label}
      </button>
    {/each}
  </div>
</div>

<style>
  .section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-top: 16px;
    margin-top: 4px;
    border-top: 1px solid var(--line);
  }

  .hint {
    font-size: 11px;
    color: var(--faint);
    line-height: 1.5;
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

  .chip:hover {
    color: var(--text);
    background: var(--bg-raise);
  }

  .chip.selected {
    color: var(--accent);
    background: var(--accent-soft);
  }

  .chip:focus-visible {
    box-shadow: 0 0 0 2px var(--accent-soft), 0 0 0 1px var(--accent);
  }
</style>
