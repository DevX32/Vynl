<script lang="ts">
  import { fade } from "svelte/transition";
  import { X } from "lucide-svelte";
  import { t } from "@lib/i18n";
  import { closeEq } from "@state/equalizer.svelte";
  import Equalizer from "./Equalizer.svelte";
  import CrossfadeControl from "./CrossfadeControl.svelte";
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" transition:fade={{ duration: 150 }} onclick={closeEq}>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div
    class="panel"
    role="dialog"
    aria-modal="true"
    aria-label={t("settings.equalizer")}
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <div class="head">
      <span class="title display">{t("settings.equalizer")}</span>
      <button class="close" onclick={closeEq} aria-label={t("common.close")}>
        <X size={16} stroke-width={1.5} />
      </button>
    </div>
    <div class="body">
      <Equalizer />
      <CrossfadeControl />
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 500;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(4px);
  }

  .panel {
    width: min(500px, calc(100vw - 40px));
    max-height: 80vh;
    background: var(--bg-raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: eq-in 0.2s cubic-bezier(0.25, 0.8, 0.25, 1);
  }

  @keyframes eq-in {
    from {
      opacity: 0;
      transform: translateY(-6px) scale(0.97);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 14px 18px;
    border-bottom: 1px solid var(--line);
  }

  .title {
    font-size: 16px;
    font-weight: 600;
  }

  .close {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line);
    color: var(--faint);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transform: rotate(45deg);
    transition:
      color 0.15s,
      border-color 0.15s,
      background 0.15s;
  }

  .close:hover {
    color: var(--text);
    border-color: var(--line-strong);
  }

  .close :global(svg) {
    transform: rotate(-45deg);
  }

  .body {
    padding: 4px 18px 18px;
    overflow-y: auto;
    scrollbar-width: none;
  }

  .body::-webkit-scrollbar {
    display: none;
  }
</style>
