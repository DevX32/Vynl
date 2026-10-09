<script lang="ts">
  import { fade } from "svelte/transition";
  import { X } from "lucide-svelte";
  import { t } from "@lib/i18n";
  import {
    SHORTCUT_CATEGORIES,
    bindingParts,
    closeShortcutsOverlay,
    getShortcutActions,
    type KeybindAction,
  } from "@state/shortcuts.svelte";

  const actions = getShortcutActions();

  const categories = SHORTCUT_CATEGORIES.map((key) => ({
    key,
    items: actions.filter((a: KeybindAction) => a.categoryKey === key),
  })).filter((c) => c.items.length > 0);
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="overlay" transition:fade={{ duration: 150 }} onclick={closeShortcutsOverlay}>
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="panel"
    role="dialog"
    aria-modal="true"
    aria-label={t("shortcuts.title")}
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <div class="head">
      <span class="title display">{t("shortcuts.title")}</span>
      <button class="close" onclick={closeShortcutsOverlay} aria-label={t("common.close")}>
        <X size={16} stroke-width={1.5} />
      </button>
    </div>
    <div class="body">
      {#each categories as cat}
        <div class="cat">
          <div class="cat-label mono">{t(cat.key)}</div>
          {#each cat.items as shortcut (shortcut.id)}
            <div class="row">
              <span class="keys">
                {#each bindingParts(shortcut.binding) as part, i (i)}
                  {#if i > 0}<span class="plus mono">+</span>{/if}
                  <kbd class="key mono">{part}</kbd>
                {/each}
              </span>
              <span class="desc mono">{t(shortcut.labelKey)}</span>
            </div>
          {/each}
        </div>
      {/each}
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
    width: min(420px, calc(100vw - 40px));
    max-height: 80vh;
    background: var(--bg-raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: shortcuts-in 0.2s cubic-bezier(0.25, 0.8, 0.25, 1);
  }

  @keyframes shortcuts-in {
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
    padding: 14px 18px;
    overflow-y: auto;
    scrollbar-width: none;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .body::-webkit-scrollbar {
    display: none;
  }

  .cat {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .cat-label {
    font-size: 10px;
    color: var(--faint);
    letter-spacing: 0.12em;
    text-transform: capitalize;
    margin-bottom: 2px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .keys {
    display: flex;
    align-items: center;
    gap: 3px;
    flex-shrink: 0;
    min-width: 104px;
  }

  .plus {
    font-size: 10px;
    color: var(--faint);
  }

  .key {
    min-width: 26px;
    padding: 3px 8px;
    background: var(--bg-raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    font-size: 11px;
    color: var(--dim);
    text-align: center;
    flex-shrink: 0;
  }

  .desc {
    font-size: 12px;
    color: var(--text);
  }
</style>
