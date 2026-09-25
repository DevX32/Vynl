<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    variant = "ghost",
    size = "md",
    disabled = false,
    title,
    onclick,
    children,
  }: {
    variant?: "primary" | "ghost" | "danger";
    size?: "sm" | "md";
    disabled?: boolean;
    title?: string;
    onclick?: () => void;
    children: Snippet;
  } = $props();
</script>

<button
  class="btn {variant} {size}"
  {disabled}
  {title}
  onclick={() => onclick?.()}
>
  {@render children()}
</button>

<style>
  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 12px;
    letter-spacing: 0.04em;
    border: 1px solid transparent;
    cursor: pointer;
    white-space: nowrap;
    transition:
      background 0.15s,
      border-color 0.15s,
      color 0.15s,
      transform 0.1s,
      box-shadow 0.15s;
  }

  button:active:not(:disabled) {
    transform: scale(0.97);
  }

  button:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .sm {
    padding: 5px 12px;
    font-size: 11px;
  }

  .md {
    padding: 8px 16px;
  }

  .primary {
    gap: 8px;
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 500;
    border-color: color-mix(in srgb, var(--accent) 35%, transparent);
    box-shadow: none;
  }

  .primary:not(:disabled):hover {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
    color: var(--accent);
    box-shadow: none;
  }

  .primary:not(:disabled):active {
    box-shadow: none;
  }

  .ghost {
    background: transparent;
    border-color: var(--line-strong);
    color: var(--dim);
  }

  .ghost:not(:disabled):hover {
    border-color: var(--faint);
    color: var(--text);
    background: var(--bg-raise);
  }

  .ghost:not(:disabled):active {
    background: rgba(255, 255, 255, 0.04);
  }

  .danger {
    gap: 8px;
    background: rgba(255, 107, 97, 0.08);
    color: var(--red);
    font-weight: 500;
    border-color: rgba(255, 107, 97, 0.35);
    box-shadow: none;
  }

  .danger:not(:disabled):hover {
    background: rgba(255, 107, 97, 0.15);
    border-color: var(--red);
    color: var(--red);
    box-shadow: none;
  }

  .danger:not(:disabled):active {
    box-shadow: none;
  }
</style>
