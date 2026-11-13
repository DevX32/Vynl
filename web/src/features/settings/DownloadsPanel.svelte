<script lang="ts">
  import { Trash2 } from "lucide-svelte";
  import { t } from "@lib/i18n";
  import { vynl } from "@lib/vynl";
  import { toasts } from "@lib/toast";
  import { refreshLibrary } from "@state/library.svelte";
  import { refreshPlaylists } from "@state/playlists.svelte";
  import Button from "@components/Button.svelte";
  import Dialog from "@components/Dialog.svelte";
  import SettingRow from "./SettingRow.svelte";

  let confirming = $state(false);
  let deleting = $state(false);

  async function removeAll(): Promise<void> {
    confirming = false;
    if (deleting) return;
    deleting = true;
    try {
      const result = await vynl.deleteAllLibraryTracks();
      await Promise.all([refreshLibrary(), refreshPlaylists()]);
      if (result.failed > 0) {
        toasts.error(
          t("downloads.removeDownloadedPartial", { failed: result.failed }),
        );
      }
      toasts.success(
        t("downloads.removeDownloadedDone", { deleted: result.deleted }),
      );
    } catch (error) {
      toasts.error(
        t("downloads.removeDownloadedFailed", {
          error: error instanceof Error ? error.message : String(error),
        }),
      );
    } finally {
      deleting = false;
    }
  }
</script>

<div class="panel">
  <div class="section">
    <div class="label mono">{t("downloads.title")}</div>
    <div class="hint mono">{t("downloads.hint")}</div>
  </div>

  <div class="group">
    <SettingRow
      label={t("downloads.removeAll")}
      hint={t("downloads.removeAllHint")}
    >
      {#snippet control()}
        <Button variant="danger" size="sm" disabled={deleting} onclick={() => (confirming = true)}>
          <Trash2 size={13} stroke-width={1.5} />
          {deleting ? t("downloads.removing") : t("downloads.removeAllConfirm")}
        </Button>
      {/snippet}
    </SettingRow>
  </div>
</div>

{#if confirming}
  <Dialog
    open
    title={t("downloads.removeAllTitle")}
    description={t("downloads.removeAllBody")}
    confirmLabel={t("downloads.removeAllConfirm")}
    danger
    onconfirm={() => void removeAll()}
    oncancel={() => (confirming = false)}
  />
{/if}

<style>
  .panel {
    display: flex;
    flex-direction: column;
    margin-bottom: 18px;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    background: var(--surface);
    overflow: hidden;
  }

  .section {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .group {
    border-top: 1px solid var(--line);
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
  }
</style>
