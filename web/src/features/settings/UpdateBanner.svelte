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

  const visible = $derived(!!status.ready || !!status.error);

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

{#if visible}
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
      {:else}
        <span class="title mono">{t("update.failedTitle")}</span>
        <span class="hint mono">{status.error}</span>
      {/if}
    </div>
    {#if status.ready}
      <Button variant="primary" size="sm" onclick={restartAndInstall}
        >{t("update.restartInstall")}</Button
      >
    {:else}
      <Button
        variant="ghost"
        size="sm"
        onclick={() => void checkForUpdate(true)}
        disabled={checking}
      >
        {checking ? t("update.checking") : t("update.retry")}
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
</style>
