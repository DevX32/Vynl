<script lang="ts">
  import { Check } from "lucide-svelte";
  import { THEMES, type ThemeId } from "@lib/theme";

  let {
    value,
    accent,
    label,
    onchange,
  }: {
    value: ThemeId;
    accent: string;
    label: string;
    onchange: (id: ThemeId) => void;
  } = $props();
</script>

<div class="theme-picker" role="group" aria-label={label}>
  {#each THEMES as theme (theme.id)}
    {@const selected = theme.id === value}
    <button
      type="button"
      class="theme-card"
      class:selected
      onclick={() => onchange(theme.id)}
      aria-pressed={selected}
    >
      <span
        class="preview"
        style:background={theme.colors.bg}
        style:border-color={theme.colors.lineStrong}
        aria-hidden="true"
      >
        <span class="pv-sidebar" style:background={theme.colors.surface}>
          <span class="pv-logo" style:background={accent}></span>
          <span class="pv-line" style:background={theme.colors.faint}></span>
          <span class="pv-line short" style:background={theme.colors.faint}></span>
        </span>
        <span class="pv-main">
          <span class="pv-title" style:background={theme.colors.text}></span>
          <span class="pv-cover" style:background={theme.colors.bgRaise}>
            <span class="pv-cover-dot" style:background={accent}></span>
          </span>
          <span class="pv-line" style:background={theme.colors.dim}></span>
        </span>
      </span>

      <span class="theme-name">
        <span class="mono">{theme.name}</span>
        {#if selected}
          <span class="check">
            <Check size={12} stroke-width={2.5} />
          </span>
        {/if}
      </span>
    </button>
  {/each}
</div>

<style>
  .theme-picker {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
    gap: 8px;
  }

  .theme-card {
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 7px;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    background: var(--bg);
    text-align: left;
    transition:
      background 0.15s,
      border-color 0.15s,
      transform 0.1s;
  }

  .theme-card:hover {
    background: var(--bg-raise);
    border-color: var(--line-strong);
  }

  .theme-card:active {
    transform: scale(0.98);
  }

  .theme-card.selected {
    background: var(--accent-soft);
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
    box-shadow: inset 0 0 0 1px
      color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .preview {
    display: flex;
    width: 100%;
    height: 60px;
    border: 1px solid;
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .pv-sidebar {
    width: 31%;
    padding: 8px 6px;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .pv-logo {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    margin-bottom: 2px;
    flex-shrink: 0;
  }

  .pv-line {
    width: 100%;
    height: 3px;
    border-radius: var(--radius-xs);
    opacity: 0.75;
  }

  .pv-line.short {
    width: 62%;
  }

  .pv-main {
    flex: 1;
    min-width: 0;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .pv-title {
    width: 66%;
    height: 5px;
    border-radius: var(--radius-xs);
    opacity: 0.85;
  }

  .pv-cover {
    flex: 1;
    min-height: 14px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .pv-cover-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    opacity: 0.9;
  }

  .pv-main .pv-line {
    width: 76%;
  }

  .theme-name {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    min-height: 14px;
    padding: 0 1px;
    font-size: 11px;
    color: var(--dim);
  }

  .theme-card.selected .theme-name {
    color: var(--text);
  }

  .check {
    display: flex;
    color: var(--accent);
    flex-shrink: 0;
  }
</style>
