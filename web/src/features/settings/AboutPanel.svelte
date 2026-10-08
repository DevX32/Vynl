<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "@lib/i18n";
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
    restartAndInstall,
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
  !checking && !failed && getUpdateCheckedAt() !== null && !available && !ready,
);

  onMount(() => {
    void initAppUpdate();
  });

  function rowLabel(): string {
    if (ready) return t("update.readyTitle");
    if (available) {
      return t("update.updateTo", { version: status?.version ?? "" });
    }
    return t("update.checkUpdates");
  }

  function rowHint(): string {
    if (failed) return status?.error ?? "";
    if (ready) return t("update.readySub", { version: status?.version ?? "" });
    if (downloading) return t("update.downloadingTitle");
    if (available) return t("update.updateAvailableHint");
    if (checking) return t("update.checking");
    if (upToDate) return t("update.upToDate");
    return "";
  }

  function chipLabel(): string {
    if (installing || checking) return t("update.checking");
    if (available) return t("update.updateButton");
    return t("update.check");
  }
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
    label={rowLabel()}
    hint={rowHint()}
  >
    {#snippet control()}
      {#if downloading}
        <div class="bar">
          <div class="fill" style:width={`${status?.progress ?? 0}%`}></div>
        </div>
      {:else if ready}
        <Button
          variant="primary"
          size="sm"
          onclick={restartAndInstall}
        >
          {t("update.restartInstall")}
        </Button>
      {:else if available}
        <Button
          variant="primary"
          size="sm"
          onclick={() => void installAppUpdate()}
          disabled={busy}
        >
          {chipLabel()}
        </Button>
      {:else}
        <Button
          size="sm"
          onclick={() => void checkForAppUpdate(true, true)}
          disabled={busy}
        >
          {chipLabel()}
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
    box-shadow: var(--shadow-sm);
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

  .bar {
    width: 100px;
    height: 3px;
    background: var(--line-strong);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--accent);
    border-radius: var(--radius-sm);
    transition: width 0.2s;
  }
</style>