<script lang="ts">
  import { X } from "lucide-svelte";
  import { fade, fly } from "svelte/transition";
  import { cubicIn, cubicOut } from "svelte/easing";
  import { t } from "@lib/i18n";
  import { focusTrap } from "@lib/actions/focus-trap";
  import { splitWikiAttribution, blobSrc } from "@lib/format";

  let {
    heading,
    text,
    image = null,
    subtitle = null,
    onClose,
  }: {
    heading: string;
    text: string;
    image?: string | null;
    subtitle?: string | null;
    onClose: () => void;
  } = $props();

  const wiki = $derived(splitWikiAttribution(text));

  function handleKey(e: KeyboardEvent): void {
    if (e.key === "Escape" && !e.defaultPrevented) {
      e.preventDefault();
      onClose();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="overlay"
  transition:fade={{ duration: 150 }}
  onclick={onClose}
  role="presentation"
>
  <div
    class="dialog"
    in:fly={{ y: 8, duration: 180, easing: cubicOut }}
    out:fly={{ y: 8, duration: 140, easing: cubicIn }}
    onclick={(e) => e.stopPropagation()}
    onkeydown={handleKey}
    role="dialog"
    aria-modal="true"
    aria-label={heading}
    tabindex="-1"
    use:focusTrap
  >
    <header class="head">
      {#if image}
        <img
          class="head-image"
          use:blobSrc={image}
          alt=""
          draggable="false"
        />
      {/if}
      <div class="head-text">
        <span class="head-kicker mono">{heading}</span>
        {#if subtitle}
          <span class="head-title">{subtitle}</span>
        {/if}
      </div>
      <button class="icon-btn close-btn" onclick={onClose} aria-label={t("common.close")}>
        <X size={12} stroke-width={1.6} />
      </button>
    </header>
    <div class="body">
      <p class="text">{wiki.body}</p>
      {#if wiki.attribution}
        <span class="source mono">{wiki.attribution}</span>
      {/if}
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: var(--z-float);
    backdrop-filter: blur(var(--overlay-blur));
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
    gap: 12px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--line);
    flex-shrink: 0;
  }

  .head-image {
    width: 44px;
    height: 44px;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
    object-fit: cover;
    display: block;
    background: var(--bg-raise);
    box-shadow: var(--shadow-sm);
  }

  .head-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .head-kicker {
    font-size: 9px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--faint);
  }

  .head-title {
    font-size: 16px;
    line-height: 1.15;
    font-weight: 600;
    letter-spacing: -0.01em;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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

  .source {
    display: block;
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--line);
    font-size: 9px;
    line-height: 1.5;
    letter-spacing: 0.02em;
    color: var(--faint);
    user-select: text;
    overflow-wrap: anywhere;
  }
</style>
