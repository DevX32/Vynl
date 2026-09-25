<script lang="ts">
  import { fade } from "svelte/transition";
  import { X } from "lucide-svelte";
  import { t } from "@lib/i18n";
  import { SHORTCUTS, closeShortcutsOverlay } from "@state/shortcuts.svelte";

  const categories = [...new Set(SHORTCUTS.map((s) => s.category))];

  function formatKey(key: string): string {
    if (key === " ") return "Space";
    if (key === "ArrowUp") return "↑";
    if (key === "ArrowDown") return "↓";
    if (key === "ArrowLeft") return "←";
    if (key === "ArrowRight") return "→";
    return key;
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="overlay" transition:fade={{ duration: 150 }} onclick={closeShortcutsOverlay}>
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="panel" onclick={(e) => e.stopPropagation()}>
    <div class="head">
      <span class="title display">{t("shortcuts.title")}</span>
      <button class="close" onclick={closeShortcutsOverlay} aria-label={t("dialog.cancel")}>
        <X size={14} stroke-width={1.5} />
      </button>
    </div>
    <div class="body">
      {#each categories as cat}
        <div class="cat">
          <div class="cat-label mono">{cat}</div>
          {#each SHORTCUTS.filter((s) => s.category === cat) as shortcut}
            <div class="row">
              <kbd class="key mono">{formatKey(shortcut.key)}</kbd>
              <span class="desc mono">{shortcut.label}</span>
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
    width: 380px;
    max-height: 70vh;
    background: var(--bg-raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px;
    border-bottom: 1px solid var(--line);
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
    transition: color 0.15s, background 0.15s;
  }

  .close:hover {
    color: var(--text);
    background: var(--bg-raise);
  }

  .body {
    padding: 14px 18px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
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

  .key {
    min-width: 40px;
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
