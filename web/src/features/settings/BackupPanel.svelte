<script lang="ts">
  import { onMount } from "svelte";
  import { Download, Upload, RefreshCw } from "lucide-svelte";
  import { t } from "@lib/i18n";
  import { vynl } from "@lib/vynl";
  import { toasts } from "@lib/toast";
  import { getCurrentPlaylists, refreshPlaylists } from "@state/playlists.svelte";
  import { queueRestore } from "@state/vault-restore.svelte";
  import Button from "@components/Button.svelte";
  import SettingRow from "./SettingRow.svelte";
  import type { ExportSummary } from "@lib/types";

  let {
    onOpenVault,
  }: {
    onOpenVault?: () => void;
  } = $props();

  let exporting = $state(false);
  let importing = $state(false);
  let cloudBackingUp = $state(false);
  let cloudRestoring = $state(false);
  let signedIn = $state(false);
  let lastExport = $state<ExportSummary | null>(null);
  let trackCount = $state<number | null>(null);
  let libraryError = $state<string | null>(null);

  const playlistCount = $derived(getCurrentPlaylists().length);

  const busy = $derived(exporting || trackCount === null || trackCount === 0);

  const exportHint = $derived.by(() => {
    if (libraryError) return libraryError;
    const base =
      trackCount === null
        ? t("common.loading")
        : trackCount === 0
          ? t("backup.nothingToExport")
          : t("backup.contentsHint", {
              tracks: trackCount,
              playlists: playlistCount,
            });
    if (!lastExport) return base;
    if (lastExport.playlistCount > 0) {
      return `${base} ${t("backup.lastExportPlaylists", {
        tracks: lastExport.trackCount,
        playlists: lastExport.playlistCount,
        bytes: fmtBytes(lastExport.bytes),
        format: lastExport.format.toUpperCase(),
      })}`;
    }
    return `${base} ${t("backup.lastExport", {
      tracks: lastExport.trackCount,
      bytes: fmtBytes(lastExport.bytes),
      format: lastExport.format.toUpperCase(),
    })}`;
  });

  async function loadLibrary(): Promise<void> {
    libraryError = null;
    trackCount = null;
    try {
      const [tracks] = await Promise.all([vynl.getLibrary(), refreshPlaylists()]);
      trackCount = tracks.length;
    } catch {
      trackCount = 0;
      libraryError = t("backup.libraryLoadFailed");
    }
  }

  onMount(() => {
    let cancelled = false;
    void (async () => {
      await loadLibrary();
      try {
        const status = await vynl.SyncStatus();
        signedIn = status.signedIn;
      } catch {
        signedIn = false;
      }
      if (cancelled) return;
    })();
    return () => {
      cancelled = true;
    };
  });

  function fmtBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  async function runImport(): Promise<void> {
    if (importing) return;
    importing = true;
    try {
      const restored = await vynl.importBackup();
      if (!restored) return;
      const confirmed = window.confirm(
        t("backup.restoreConfirm", { tracks: restored.total }),
      );
      if (!confirmed) return;
      queueRestore(
        {
          ...restored,
          collection: {
            ...restored.collection,
            title: t("backup.restoredTitle"),
          },
        },
        true,
      );
      toasts.success(
        t("backup.restored", {
          tracks: restored.total,
          known: restored.knownSources,
        }),
        6000,
      );
      onOpenVault?.();
    } catch (e) {
      toasts.error(
        t("backup.restoreFailed", {
          error: e instanceof Error ? e.message : String(e),
        }),
        6000,
      );
    } finally {
      importing = false;
    }
  }

  async function runExport(): Promise<void> {
    if (exporting) return;
    exporting = true;
    try {
      const result = await vynl.exportBackup();
      if (!result) return;
      lastExport = result;
      const params = {
        tracks: result.trackCount,
        playlists: result.playlistCount,
        path: result.path,
      };
      toasts.success(
        result.playlistCount > 0
          ? t("backup.savedPlaylists", params)
          : t("backup.saved", params),
        6000,
      );
    } catch (e) {
      toasts.error(
        t("backup.failed", {
          error: e instanceof Error ? e.message : String(e),
        }),
        6000,
      );
    } finally {
      exporting = false;
    }
  }

  async function runCloudBackup(): Promise<void> {
    if (cloudBackingUp) return;
    cloudBackingUp = true;
    try {
      await vynl.backupToCloud();
      toasts.success(t("backup.cloudSaved"), 6000);
    } catch (e) {
      toasts.error(
        t("backup.cloudFailed", {
          error: e instanceof Error ? e.message : String(e),
        }),
        6000,
      );
    } finally {
      cloudBackingUp = false;
    }
  }

  async function runCloudRestore(): Promise<void> {
    if (cloudRestoring) return;
    cloudRestoring = true;
    try {
      const restored = await vynl.restoreFromCloud();
      if (!restored) return;
      const confirmed = window.confirm(
        t("backup.restoreConfirm", { tracks: restored.total }),
      );
      if (!confirmed) return;
      queueRestore(
        {
          ...restored,
          collection: {
            ...restored.collection,
            title: t("backup.restoredTitle"),
          },
        },
        true,
      );
      toasts.success(
        t("backup.restored", {
          tracks: restored.total,
          known: restored.knownSources,
        }),
        6000,
      );
      onOpenVault?.();
    } catch (e) {
      toasts.error(
        t("backup.restoreFailed", {
          error: e instanceof Error ? e.message : String(e),
        }),
        6000,
      );
    } finally {
      cloudRestoring = false;
    }
  }
</script>

<div class="panel">
  <div class="group-head first">
    <span class="label mono">{t("backup.title")}</span>
  </div>
  <div class="section flush">
    <div class="hint mono">{t("backup.hint")}</div>
  </div>

  <div class="rows">
    <SettingRow label={t("backup.contents")} hint={exportHint}>
      {#snippet control()}
        <div class="export-controls">
          {#if libraryError}
            <Button size="sm" variant="ghost" onclick={() => void loadLibrary()}>
              <RefreshCw size={13} stroke-width={1.5} />
              {t("common.retry")}
            </Button>
          {:else}
            <Button size="sm" disabled={busy} onclick={runExport}>
              <Download size={13} stroke-width={1.5} />
              {exporting ? t("backup.exporting") : t("backup.export")}
            </Button>
          {/if}
        </div>
      {/snippet}
    </SettingRow>

    <SettingRow
      label={t("backup.restoreTitle")}
      hint={t("backup.restoreHint")}
    >
      {#snippet control()}
        <Button size="sm" disabled={importing} onclick={runImport}>
          <Upload size={13} stroke-width={1.5} />
          {importing ? t("backup.restoring") : t("backup.restore")}
        </Button>
      {/snippet}
    </SettingRow>

    {#if signedIn}
      <SettingRow
        label={t("backup.cloudTitle")}
        hint={t("backup.cloudHint")}
      >
        {#snippet control()}
          <div class="export-controls">
            <Button
              size="sm"
              disabled={cloudBackingUp}
              onclick={runCloudBackup}
            >
              <Upload size={13} stroke-width={1.5} />
              {cloudBackingUp ? t("backup.cloudBackingUp") : t("backup.cloudBackup")}
            </Button>
            <Button
              size="sm"
              disabled={cloudRestoring}
              onclick={runCloudRestore}
            >
              <Download size={13} stroke-width={1.5} />
              {cloudRestoring ? t("backup.cloudRestoring") : t("backup.cloudRestore")}
            </Button>
          </div>
        {/snippet}
      </SettingRow>
    {/if}
  </div>

</div>

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

  .section.flush {
    padding-top: 10px;
  }

  .group-head {
    padding: 14px 16px 0;
    display: flex;
    align-items: center;
  }

  .panel > .group-head.first {
    padding-top: 16px;
  }

  .rows {
    border-top: 1px solid var(--line);
  }

  .rows > :global(.row + .row) {
    border-top: 1px solid var(--line);
  }

  .export-controls {
    display: flex;
    align-items: center;
    gap: 8px;
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
