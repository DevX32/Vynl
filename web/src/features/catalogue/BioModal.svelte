<script lang="ts">
  import { X } from "lucide-svelte";
  import { t } from "@lib/i18n";

  let {
    heading,
    text,
    onClose,
  }: {
    heading: string;
    text: string;
    onClose: () => void;
  } = $props();

  let dialogEl = $state<HTMLDivElement>();

  $effect(() => {
    dialogEl?.focus();
  });

  function handleKey(e: KeyboardEvent): void {
    if (e.key === "Escape" && !e.defaultPrevented) {
      e.preventDefault();
      onClose();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="overlay" onclick={onClose} role="presentation">
  <div
    class="dialog"
    bind:this={dialogEl}
    onclick={(e) => e.stopPropagation()}
    onkeydown={handleKey}
    role="dialog"
    aria-modal="true"
    aria-label={heading}
    tabindex="-1"
  >
    <header class="head">
      <span class="title mono">{heading}</span>
      <button class="close" onclick={onClose} aria-label={t("common.close")}>
        <X size={14} stroke-width={1.5} />
      </button>
    </header>
    <div class="body">
      <p class="text">{text}</p>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    backdrop-filter: blur(4px);
  }

  .dialog {
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    width: 480px;
    max-width: calc(100vw - 48px);
    max-height: min(70vh, 560px);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
    outline: none;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--line);
    flex-shrink: 0;
  }

  .title {
    font-size: 10px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--faint);
  }

  .close {
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    color: var(--dim);
    cursor: pointer;
    transition:
    background 0.12s,
    color 0.12s;
  }

  .close:hover {
    background: var(--bg-raise);
    color: var(--text);
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 16px 18px 18px;
  }

  .text {
    margin: 0;
    font-size: 13px;
    line-height: 1.7;
    color: var(--dim);
    white-space: pre-line;
    overflow-wrap: anywhere;
    user-select: text;
  }
</style>
