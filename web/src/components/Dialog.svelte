<script lang="ts">
  import type { Snippet } from "svelte";
  import Button from "./Button.svelte";
  import { t } from "@lib/i18n";

  let {
    open = false,
    title = "",
    description = "",
    confirmLabel = "Confirm",
    danger = false,
    icon,
    onconfirm,
    oncancel,
    children,
  }: {
    open?: boolean;
    title?: string;
    description?: string;
    confirmLabel?: string;
    danger?: boolean;
    icon?: typeof import("lucide-svelte").Pencil;
    onconfirm?: () => void;
    oncancel: () => void;
    children?: Snippet;
  } = $props();

  function backdrop() {
    oncancel();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      oncancel();
    }
  }
</script>

{#if open}
  <div
    class="backdrop"
    role="presentation"
    onclick={backdrop}
  >
    <div
      class="panel"
      role="dialog"
      aria-modal="true"
      aria-labelledby="dlg-title"
      aria-describedby={description ? "dlg-desc" : undefined}
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={onKey}
    >
      <div class="header">
        {#if icon}
          {@const Icon = icon}
          <span class="header-icon"><Icon size={14} stroke-width={1.5} /></span>
        {/if}
        <span id="dlg-title">{title}</span>
      </div>
      {#if description}
        <p id="dlg-desc" class="desc">{description}</p>
      {/if}
      {#if children}
        <div class="body">
          {@render children()}
        </div>
      {/if}
      <div class="footer">
        <Button onclick={oncancel}>{t("dialog.cancel")}</Button>
        <Button variant={danger ? "danger" : "primary"} onclick={onconfirm}
          >{confirmLabel || t("dialog.confirm")}</Button
        >
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(6px);
    animation: fade-in 0.12s cubic-bezier(0.25, 0.08, 0.25, 1);
  }

  @keyframes fade-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  .panel {
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 20px;
    min-width: 280px;
    max-width: 380px;
    position: relative;
    animation: slide-in 0.12s cubic-bezier(0.25, 0.08, 0.25, 1);
    box-shadow:
      0 22px 60px rgba(0, 0, 0, 0.55),
      inset 0 1px 0 rgba(255, 255, 255, 0.03);
  }

  @keyframes slide-in {
    from {
      opacity: 0;
      transform: scale(0.96) translateY(6px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }

  .header {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    margin-bottom: 10px;
    position: relative;
  }

  .header:has(+ .desc) {
    margin-bottom: 4px;
  }

  .header-icon {
    display: inline-flex;
    align-items: center;
    color: var(--accent);
    flex-shrink: 0;
  }

  .desc {
    margin: 0 0 12px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--dim);
  }

  .body {
    font-size: 12px;
    color: var(--dim);
    line-height: 1.5;
    margin-bottom: 16px;
  }

  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

</style>
