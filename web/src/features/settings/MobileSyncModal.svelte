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
    getMobileSyncRotating,
    getMobileSyncStatus,
    rotateMobileSyncPin,
    setMobileSyncEnabled,
  } from "@state/mobile-sync.svelte";

  const status = $derived(getMobileSyncStatus());
  const busy = $derived(getMobileSyncBusy());
  const rotating = $derived(getMobileSyncRotating());
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
      <button class="icon-btn" onclick={closeMobileSyncUi} aria-label={t("common.close")}>
        <X size={16} stroke-width={1.5} />
      </button>
    </div>

    <div class="body">
      {#if loading && !status}
        <div class="empty mono">{t("settings.mobileSyncLoading")}</div>
      {:else if !status}
        <div class="empty mono">{t("settings.mobileSyncUnavailable")}</div>
      {:else}
        <div class="row-toggle">
          <div class="text">
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
          {#if !status.running}
            <div class="notice warn">
              <WifiOff size={14} stroke-width={1.5} />
              <span class="mono">{t("settings.mobileSyncNotRunning")}</span>
            </div>
          {/if}

          <div class="qr-wrap">
              {#if status.qrSvg}
                <div class="qr">{@html status.qrSvg}</div>
                <div class="qr-cap mono">{t("settings.mobileSyncQr")}</div>
              {:else}
                <div class="qr qr-empty mono">
                  {t("settings.mobileSyncNoNetwork")}
                </div>
              {/if}

              <div class="qr-actions">
                <Button
                  size="sm"
                  onclick={() => void rotateMobileSyncPin()}
                  disabled={busy}
                >
                  <span class="qr-icon">
                    <RefreshCw size={13} stroke-width={1.5} class={rotating ? "spin" : ""} />
                  </span>
                  {t("settings.mobileSyncNewQr")}
                </Button>
              </div>
            </div>

          <div class="notice insecure">
            <ShieldAlert size={14} stroke-width={1.5} />
            <span class="mono">{t("settings.mobileSyncInsecureNotice")}</span>
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

  .title {
    font-size: 16px;
    font-weight: 600;
    white-space: nowrap;
  }

  .icon-btn {
    width: 28px;
    height: 28px;
    flex-shrink: 0;
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

  .icon-btn:hover {
    color: var(--text);
    border-color: var(--line-strong);
  }

  .icon-btn :global(svg) {
    transform: rotate(-45deg);
  }

  .qr-icon {
    display: inline-flex;
    align-items: center;
  }

  .qr-actions :global(button:disabled) {
    opacity: 0.75;
  }

  .qr-icon :global(.spin) {
    animation: msync-spin 0.7s linear infinite;
  }

  @keyframes msync-spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .qr-icon :global(.spin) {
      animation-duration: 2.4s;
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
    margin-bottom: 14px;
    border-bottom: 1px solid var(--line);
  }

  .text {
    min-width: 0;
  }

  .hint {
    font-size: 11px;
    color: var(--dim);
    line-height: 1.5;
  }

  .notice {
    display: flex;
    align-items: flex-start;
    gap: 8px;
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

  .notice.insecure {
    margin-top: 16px;
  }

  .qr-wrap {
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
    width: 200px;
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

  .foot {
    margin-top: 16px;
    padding-top: 14px;
    border-top: 1px solid var(--line);
    font-size: 11px;
    color: var(--faint);
    line-height: 1.6;
  }

  </style>