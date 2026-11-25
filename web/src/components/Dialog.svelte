<script lang="ts">
  import type { Snippet } from "svelte";
  import { fade, fly } from "svelte/transition";
  import { cubicIn, cubicOut } from "svelte/easing";
  import Button from "./Button.svelte";
  import { t } from "@lib/i18n";
  import { focusTrap } from "@lib/actions/focus-trap";

  let {
    open = false,
    title = "",
    description = "",
    confirmLabel = "Confirm",
    danger = false,
    confirmDisabled = false,
    onconfirm,
    oncancel,
    children,
  }: {
    open?: boolean;
    title?: string;
    description?: string;
    confirmLabel?: string;
    danger?: boolean;
    confirmDisabled?: boolean;
    onconfirm?: () => void;
    oncancel: () => void;
    children?: Snippet;
  } = $props();

  function backdrop() {
    oncancel();
  }



  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape" && !e.defaultPrevented) {
      e.preventDefault();
      e.stopPropagation();
      oncancel();
    }
  }
</script>

{#if open}
  <div
    class="backdrop"
    role="presentation"
    transition:fade={{ duration: 150 }}
    onclick={backdrop}
  >
    <div
      class="panel"
      role="dialog"
      aria-modal="true"
      aria-labelledby="dlg-title"
      aria-describedby={description ? "dlg-desc" : undefined}
      tabindex="-1"
      in:fly={{ y: 8, duration: 180, easing: cubicOut }}
      out:fly={{ y: 8, duration: 140, easing: cubicIn }}
      onclick={(e) => e.stopPropagation()}
      onkeydown={onKey}
      use:focusTrap
    >
      <div class="header">
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
        <Button onclick={oncancel}>{t("common.cancel")}</Button>
        <Button variant={danger ? "danger" : "primary"} onclick={onconfirm} disabled={confirmDisabled}
          >{confirmLabel || t("common.confirm")}</Button
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
  }

  .panel {
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 20px;
    min-width: 280px;
    max-width: 380px;
    position: relative;
    box-shadow:
      0 22px 60px rgba(0, 0, 0, 0.55),
      inset 0 1px 0 rgba(255, 255, 255, 0.03);
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
