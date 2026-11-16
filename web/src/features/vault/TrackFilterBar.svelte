<script lang="ts">
  import { t } from "@lib/i18n";

  let {
    total,
    trackFilter,
    counts,
    onFilterChange,
  }: {
    total: number;
    trackFilter: string;
    counts: { done: number; skipped: number; failed: number; queued: number; active: number };
    onFilterChange: (filter: string) => void;
  } = $props();
</script>

<div class="filter-tabs mono">
  <button
    class="filter-tab"
    class:active={trackFilter === "all"}
    onclick={() => onFilterChange("all")}
  >{total} {t("vault.all")}</button>
  {#if counts.queued > 0}
    <button
      class="filter-tab queued"
      class:active={trackFilter === "queued"}
      onclick={() => onFilterChange("queued")}
    >{counts.queued} {t("vault.queued")}</button>
  {/if}
  {#if counts.active > 0}
    <button
      class="filter-tab active-tab"
      class:active={trackFilter === "active"}
      onclick={() => onFilterChange("active")}
    >{counts.active} {t("vault.active")}</button>
  {/if}
  {#if counts.done > 0}
    <button
      class="filter-tab done"
      class:active={trackFilter === "done"}
      onclick={() => onFilterChange("done")}
    >{counts.done} {t("vault.done")}</button>
  {/if}
  {#if counts.skipped > 0}
    <button
      class="filter-tab skipped"
      class:active={trackFilter === "skipped"}
      onclick={() => onFilterChange("skipped")}
    >{counts.skipped} {t("vault.skipped")}</button>
  {/if}
  {#if counts.failed > 0}
    <button
      class="filter-tab failed"
      class:active={trackFilter === "failed"}
      onclick={() => onFilterChange("failed")}
    >{counts.failed} {t("vault.failed")}</button>
  {/if}
</div>

<style>
  .filter-tabs {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .filter-tabs::-webkit-scrollbar {
    width: 0;
    height: 0;
  }

  .filter-tab {
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
    background: transparent;
    color: var(--dim);
    font-size: 10px;
    font-family: inherit;
    letter-spacing: 0.04em;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.15s, border-color 0.15s, color 0.15s;
  }

  .filter-tab:hover {
    background: var(--bg-raise);
    border-color: var(--line-strong);
    color: var(--text);
  }

  .filter-tab.active {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
  }

  .filter-tab.queued.active {
    border-color: var(--dim);
    color: var(--dim);
  }

  .filter-tab.done.active {
    border-color: var(--green);
    color: var(--green);
  }

  .filter-tab.skipped.active {
    border-color: var(--amber);
    color: var(--amber);
  }

  .filter-tab.failed.active {
    border-color: var(--red);
    color: var(--red);
  }

  .filter-tab.active-tab.active {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
