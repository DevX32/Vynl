<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "@lib/i18n";
  import { getMobileSyncStatus, openMobileSyncUi, refreshMobileSync } from "@state/mobile-sync.svelte";

  const status = $derived(getMobileSyncStatus());
  const live = $derived(!!status?.enabled && !!status?.running);

  onMount(() => {
    void refreshMobileSync();
  });
</script>

<div class="section row-section">
  <div>
    <div class="label mono">{t("settings.mobileSync")}</div>
    <div class="hint mono">{t("settings.mobileSyncHint")}</div>
  </div>
  <div class="row-actions">
    {#if status}
      <span class="status mono" class:on={live}>
        {live
          ? t("settings.mobileSyncLive")
          : status.enabled
            ? t("settings.mobileSyncStopped")
            : t("settings.mobileSyncOff")}
      </span>
    {/if}
    <button class="chip mono" onclick={openMobileSyncUi}>
      {t("plugins.open")}
    </button>
  </div>
</div>

<style>
  .section {
    padding: 16px;
    border-top: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .row-section {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
  }

  .label {
    font-size: 11px;
    color: var(--dim);
    letter-spacing: 0.1em;
  }

  .hint {
    font-size: 11px;
    color: var(--faint);
    line-height: 1.5;
    margin-top: 4px;
  }

  .row-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .status {
    font-size: 10.5px;
    color: var(--faint);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    transition: color 0.15s;
  }

  .status.on {
    color: var(--accent);
  }

  .chip {
    font-size: 11px;
    color: var(--dim);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    padding: 4px 10px;
    cursor: pointer;
    transition:
      color 0.15s,
      background 0.15s;
  }

  .chip:hover {
    color: var(--text);
    background: var(--bg-raise);
  }
</style>
