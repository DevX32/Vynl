<script lang="ts">
  import { t } from "@lib/i18n";
  import { fmtDuration } from "@lib/format";
  import Button from "@components/Button.svelte";
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
      <Button
        size="sm"
        selected={opt.secs === current}
        onclick={() => choose(opt.secs)}
      >
        {opt.label}
      </Button>
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
</style>
