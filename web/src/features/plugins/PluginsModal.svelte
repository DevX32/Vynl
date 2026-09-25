<script lang="ts">
  import { fade } from "svelte/transition";
  import { X } from "lucide-svelte";
  import { t } from "@lib/i18n";
  import { closePluginsUi } from "@state/plugins.svelte";
  import PluginsPage from "./PluginsPage.svelte";
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" transition:fade={{ duration: 150 }} onclick={closePluginsUi}>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div
    class="panel"
    role="dialog"
    aria-modal="true"
    aria-label={t("plugins.title")}
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <div class="head">
      <span class="title display">{t("plugins.title")}</span>
      <button class="close" onclick={closePluginsUi} aria-label={t("common.close")}>
        <X size={14} stroke-width={1.5} />
      </button>
    </div>
    <div class="body">
      <PluginsPage />
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
    width: min(720px, calc(100vw - 40px));
    max-height: 85vh;
    background: var(--bg-raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: plugins-in 0.2s cubic-bezier(0.25, 0.8, 0.25, 1);
  }

  @keyframes plugins-in {
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
    flex-shrink: 0;
  }

  .title {
    font-size: 16px;
    font-weight: 600;
  }

  .close {
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    background: none;
    border: none;
    color: var(--faint);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition:
      color 0.15s,
      background 0.15s;
  }

  .close:hover {
    color: var(--text);
    background: var(--surface);
  }

  .body {
    flex: 1;
    min-height: 0;
    padding: 4px 18px 18px;
    overflow-y: auto;
    scrollbar-width: none;
  }

  .body::-webkit-scrollbar {
    display: none;
  }
</style>
