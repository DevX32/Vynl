<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "@lib/i18n";
  import { LoaderCircle } from "lucide-svelte";
  import Button from "@components/Button.svelte";
  import SettingRow from "./SettingRow.svelte";
  import SupportBanner from "./SupportBanner.svelte";
  import {
    getCurrentVersion,
    getUpdateAvailable,
    getUpdateCheckedAt,
    getUpdateChecking,
    getUpdateInstalling,
    getUpdateStatus,
    checkForAppUpdate,
    installAppUpdate,
    initAppUpdate,
  } from "@state/app-update.svelte";

  const status = $derived(getUpdateStatus());
  const version = $derived(getCurrentVersion());
  const available = $derived(getUpdateAvailable());
  const checking = $derived(getUpdateChecking());
  const installing = $derived(getUpdateInstalling());
  const ready = $derived(!!status?.ready);
  const downloading = $derived(!!status?.downloading);
  const failed = $derived(!!status?.error);
  const busy = $derived(checking || installing);
const upToDate = $derived(
  !failed && getUpdateCheckedAt() !== null && !available && !ready,
);

  onMount(() => {
    void initAppUpdate();
  });

  const rowLabel = $derived(
    available
      ? t("update.updateTo", { version: status?.version ?? "" })
      : t("update.checkUpdates"),
  );

  const rowHint = $derived.by(() => {
    if (failed) return status?.error ?? "";
    if (available) return t("update.updateAvailableHint");
    if (upToDate) return t("update.upToDate");
    return "";
  });

  const chipLabel = $derived(
    installing || checking
      ? t("update.checking")
      : available
        ? t("update.updateButton")
        : t("update.check"),
  );
</script>

<div class="panel">
  <div class="head">
    <div class="mark">
      <img src="/icon.png" alt="" draggable="false" />
    </div>
    <div class="id">
      <span class="name display">Vynl</span>
      <span class="ver mono">
        {version ? t("update.versionLine", { version }) : "—"}
      </span>
    </div>
  </div>

  <SettingRow
    label={rowLabel}
    hint={rowHint}
  >
    {#snippet control()}
      {#if downloading}
        <Button
          size="sm"
          disabled
          title={t("update.downloadingTitle")}
        >
          <span class="spin"><LoaderCircle size={14} stroke-width={1.5} aria-hidden="true" /></span>
          {t("update.downloadingTitle")}
        </Button>
      {:else if available}
        <Button
          variant="primary"
          size="sm"
          onclick={() => void installAppUpdate()}
          disabled={busy}
          title={busy ? t("update.checking") : undefined}
        >
          {#if busy}
            <span class="spin"><LoaderCircle size={14} stroke-width={1.5} aria-hidden="true" /></span>
          {:else}
            {chipLabel}
          {/if}
        </Button>
      {:else}
        <Button
          size="sm"
          onclick={() => void checkForAppUpdate(true, true)}
          disabled={busy}
          title={busy ? t("update.checking") : undefined}
        >
          {#if busy}
            <span class="spin"><LoaderCircle size={14} stroke-width={1.5} aria-hidden="true" /></span>
          {:else}
            {chipLabel}
          {/if}
        </Button>
      {/if}
    {/snippet}
  </SettingRow>

  <SupportBanner />
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    background: var(--surface);
    margin-bottom: 18px;
    overflow: hidden;
    transition:
      border-color 0.2s,
      background 0.2s;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--line);
  }

  .mark {
    flex-shrink: 0;
    width: 42px;
    height: 42px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .mark img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .id {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .name {
    font-size: 20px;
    font-weight: 600;
    line-height: 1.1;
    letter-spacing: -0.3px;
  }

  .ver {
    font-size: 11px;
    color: var(--faint);
  }

  .spin {
    display: inline-flex;
    animation: spin 0.85s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
