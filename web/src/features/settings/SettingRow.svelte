<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    label,
    hint,
    control,
    stacked = false,
  }: {
    label: string;
    hint?: string;
    control: Snippet;
    stacked?: boolean;
  } = $props();
</script>

<div class="row" class:stacked>
  <div class="text">
    <div class="label mono">{label}</div>
    {#if hint}<div class="hint mono">{hint}</div>{/if}
  </div>
  <div class="control">
    {@render control()}
  </div>
</div>

<style>
  .row {
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 16px;
  }

  .row.stacked {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
  }

  .text {
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
    margin-top: 4px;
  }

  .control {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .stacked .control {
    flex-wrap: wrap;
  }
</style>