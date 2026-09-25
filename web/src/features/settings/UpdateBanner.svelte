<script lang="ts">
  import { onMount } from "svelte";
  import type { UpdateStatus } from "../../lib/types";
  import { vynl } from "../../lib/vynl";
  import { t } from "@lib/i18n";
  import Button from "../../components/Button.svelte";

  let status = $state<UpdateStatus>({
    available: false,
    currentVersion: "0.1.0",
  });
  let checking = $state(false);
  let installing = $state(false);

  onMount(() => {
    void checkForUpdate();
    return vynl.onUpdateStatus((s) => {
      status = s;
    });
  });

  async function checkForUpdate(force = false) {
    if (checking) return;
    checking = true;
    try {
      status = await vynl.checkAppUpdate(force);
    } catch (e) {
      status = {
        ...status,
        downloading: false,
        ready: false,
        error: String(e),
      };
    } finally {
      checking = false;
    }
  }

  async function installUpdate() {
    if (checking || installing) return;
    installing = true;
    try {
      await vynl.installAppUpdate();
    } catch (e) {
      status = {
        ...status,
        downloading: false,
        ready: false,
        error: String(e),
      };
    } finally {
      installing = false;
    }
  }

  async function restartAndInstall() {
    try {
      await vynl.restartApp();
    } catch (e) {
      status = {
        ...status,
        downloading: false,
        ready: false,
        error: String(e),
      };
    }
  }
</script>

{#if status.available || status.downloading || status.ready || status.error}
  <div
    class="update-banner"
    class:ready={status.ready}
    class:error={status.error}
  >
    <div class="info">
      {#if status.ready}
        <span class="title mono">{t("update.readyTitle")}</span>
        <span class="hint mono"
          >{t("update.readySub", { version: status.version ?? "" })}</span
        >
      {:else if status.downloading}
        <span class="title mono">{t("update.downloadingTitle")}</span>
        <span class="hint mono">{Math.round(status.progress ?? 0)}%</span>
      {:else if status.error}
        <span class="title mono">{t("update.failedTitle")}</span>
        <span class="hint mono">{status.error}</span>
      {:else}
        <span class="title mono">{t("update.availableTitle")}</span>
        <span class="hint mono"
          >{t("update.availableSub", { version: status.version ?? "" })}</span
        >
      {/if}
    </div>
    {#if status.downloading}
      <div class="bar">
        <div class="fill" style:width={`${status.progress ?? 0}%`}></div>
      </div>
    {:else if status.ready}
      <Button variant="primary" size="sm" onclick={restartAndInstall}
        >{t("update.restartInstall")}</Button
      >
    {:else if status.error}
      <Button
        variant="ghost"
        size="sm"
        onclick={() => void checkForUpdate(true)}
        disabled={checking || installing}
      >
        {checking ? t("update.checking") : t("update.retry")}
      </Button>
    {:else}
      <Button
        variant="ghost"
        size="sm"
        onclick={installUpdate}
        disabled={checking || installing}
      >
        {checking || installing ? t("update.checking") : t("update.updateButton")}
      </Button>
    {/if}
  </div>
{/if}

<style>
  .update-banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border: 1px solid var(--line);
    border-left: 2px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--surface);
    margin-bottom: 18px;
    box-shadow: var(--shadow-sm);
  }

  .update-banner.ready {
    border-left-color: var(--green);
  }

  .update-banner.error {
    border-left-color: rgba(255, 107, 97, 0.6);
  }

  .info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title {
    font-size: 12px;
    letter-spacing: 0.1em;
    text-transform: capitalize;
  }

  .hint {
    color: var(--faint);
    font-size: 11px;
  }

  .bar {
    flex: 0 0 100px;
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
