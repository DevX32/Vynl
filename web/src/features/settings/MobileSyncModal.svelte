<script lang="ts">
  import { fade } from "svelte/transition";
  import { X, RefreshCw, ShieldAlert, WifiOff } from "lucide-svelte";
  import { t } from "@lib/i18n";
  import Button from "@components/Button.svelte";
  import Switch from "@components/Switch.svelte";
  import {
    closeMobileSyncUi,
    getMobileSyncBusy,
    getMobileSyncLoading,
    getMobileSyncStatus,
    refreshMobileSync,
    rotateMobileSyncPin,
    setMobileSyncEnabled,
  } from "@state/mobile-sync.svelte";

  const status = $derived(getMobileSyncStatus());
  const busy = $derived(getMobileSyncBusy());
  const loading = $derived(getMobileSyncLoading());

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape") closeMobileSyncUi();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" transition:fade={{ duration: 150 }} onclick={closeMobileSyncUi}>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div
    class="panel"
    role="dialog"
    aria-modal="true"
    aria-label={t("settings.mobileSync")}
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <div class="head">
      <span class="title display">{t("settings.mobileSync")}</span>
      <div class="head-actions">
        <button
          class="icon-btn"
          onclick={() => void refreshMobileSync(true)}
          disabled={loading || busy}
          aria-label={t("settings.mobileSyncRefresh")}
        >
          <RefreshCw size={13} stroke-width={1.5} class={loading ? "spin" : ""} />
        </button>
        <button class="icon-btn" onclick={closeMobileSyncUi} aria-label={t("common.close")}>
          <X size={14} stroke-width={1.5} />
        </button>
      </div>
    </div>

    <div class="body">
      {#if loading && !status}
        <div class="empty mono">{t("settings.mobileSyncLoading")}</div>
      {:else if !status}
        <div class="empty mono">{t("settings.mobileSyncUnavailable")}</div>
      {:else}
        <div class="row-toggle">
          <div>
            <div class="label mono">{t("settings.mobileSync")}</div>
            <div class="hint mono">{t("settings.mobileSyncHint")}</div>
          </div>
          <Switch
            on={status.enabled}
            disabled={busy}
            label={t("settings.mobileSyncToggle")}
            onclick={() => void setMobileSyncEnabled(!status!.enabled)}
          />
        </div>

        {#if status.enabled}
          <div class="notice">
            <ShieldAlert size={14} stroke-width={1.5} />
            <span class="mono">{t("settings.mobileSyncInsecureNotice")}</span>
          </div>

          {#if !status.running}
            <div class="notice warn">
              <WifiOff size={14} stroke-width={1.5} />
              <span class="mono">{t("settings.mobileSyncNotRunning")}</span>
            </div>
          {/if}

          <div class="qr-block">
            {#if status.qrSvg}
              <div class="qr">{@html status.qrSvg}</div>
              <div class="qr-cap mono">{t("settings.mobileSyncQr")}</div>
            {:else}
              <div class="qr qr-empty mono">{t("settings.mobileSyncNoNetwork")}</div>
            {/if}

            <div class="qr-actions">
              <Button
                size="sm"
                onclick={() => void rotateMobileSyncPin()}
                disabled={busy}
              >
                <RefreshCw size={13} stroke-width={1.5} />
                {t("settings.mobileSyncNewQr")}
              </Button>
            </div>
            <div class="qr-note mono">{t("settings.mobileSyncScanOnly")}</div>
          </div>

          <div class="foot mono">{t("settings.mobileSyncFoot")}</div>
        {/if}
      {/if}
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
    width: min(560px, calc(100vw - 40px));
    max-height: 85vh;
    background: var(--bg-raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: msync-in 0.2s cubic-bezier(0.25, 0.8, 0.25, 1);
  }

  @keyframes msync-in {
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

  .head-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .title {
    font-size: 16px;
    font-weight: 600;
    white-space: nowrap;
  }

  .icon-btn {
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

  .icon-btn:hover:not(:disabled) {
    color: var(--text);
    background: var(--surface);
  }

  .icon-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  :global(.spin) {
    animation: msync-spin 0.9s linear infinite;
  }

  @keyframes msync-spin {
    to {
      transform: rotate(360deg);
    }
  }

  .body {
    flex: 1;
    min-height: 0;
    padding: 16px 18px 18px;
    overflow-y: auto;
    scrollbar-width: none;
  }

  .body::-webkit-scrollbar {
    display: none;
  }

  .empty {
    padding: 32px 0;
    text-align: center;
    font-size: 12px;
    color: var(--faint);
  }

  .row-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--line);
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

  .notice {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin-top: 14px;
    padding: 9px 11px;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    color: var(--dim);
    font-size: 11px;
    line-height: 1.5;
  }

  .notice :global(svg) {
    flex-shrink: 0;
    margin-top: 1px;
  }

  .notice.warn {
    color: var(--amber);
    border-color: rgba(217, 164, 65, 0.3);
  }

  .qr-block {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    margin-top: 16px;
  }

  .qr {
    width: 200px;
    height: 200px;
    background: #fff;
    border-radius: var(--radius-sm);
    padding: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .qr :global(svg) {
    width: 100%;
    height: 100%;
  }

  .qr-empty {
    color: #666;
    font-size: 11px;
    text-align: center;
    padding: 12px;
  }

  .qr-cap {
    font-size: 10px;
    color: var(--faint);
    text-align: center;
    line-height: 1.4;
  }

  .qr-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .qr-note {
    font-size: 10.5px;
    color: var(--faint);
    text-align: center;
    line-height: 1.5;
    max-width: 260px;
  }

  .foot {
    margin-top: 18px;
    padding-top: 14px;
    border-top: 1px solid var(--line);
    font-size: 11px;
    color: var(--faint);
    line-height: 1.6;
  }
</style>
