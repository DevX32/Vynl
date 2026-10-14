<script lang="ts">
  import { onMount } from "svelte";
  import { Download, Upload } from "lucide-svelte";
  import { t } from "@lib/i18n";
  import { vynl } from "@lib/vynl";
  import { toasts } from "@lib/toast";
  import { getCurrentPlaylists, refreshPlaylists } from "@state/playlists.svelte";
  import { queueRestore } from "@state/vault-restore.svelte";
  import Button from "@components/Button.svelte";
  import Select from "@components/Select.svelte";
  import Switch from "@components/Switch.svelte";
  import SettingRow from "./SettingRow.svelte";
  import type { ExportFormat, ExportSummary } from "@lib/types";

  let {
    onOpenVault,
  }: {
    onOpenVault?: () => void;
  } = $props();

  let format = $state<ExportFormat>("json");
  let includePlaylists = $state(true);
  let exporting = $state(false);
  let importing = $state(false);
  let lastExport = $state<ExportSummary | null>(null);
  let trackCount = $state<number | null>(null);

  const formats = $derived([
    { value: "json", label: t("backup.formatJson") },
    { value: "csv", label: t("backup.formatCsv") },
    { value: "m3u", label: t("backup.formatM3u") },
  ]);

  const formatHint = $derived(
    format === "json"
      ? t("backup.formatJsonHint")
      : format === "csv"
        ? t("backup.formatCsvHint")
        : t("backup.formatM3uHint"),
  );

  const playlistCount = $derived(
    includePlaylists ? getCurrentPlaylists().length : 0,
  );

  const busy = $derived(exporting || trackCount === null || trackCount === 0);

  onMount(() => {
    let cancelled = false;
    void (async () => {
      try {
        const [tracks] = await Promise.all([vynl.getLibrary(), refreshPlaylists()]);
        if (!cancelled) trackCount = tracks.length;
      } catch {
        if (!cancelled) trackCount = 0;
      }
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
      queueRestore({
        ...restored,
        collection: {
          ...restored.collection,
          title: t("backup.restoredTitle"),
        },
      });
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
      const result = await vynl.exportBackup(format, includePlaylists);
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
</script>

<div class="panel">
  <div class="section">
    <div class="label mono">{t("backup.title")}</div>
    <div class="hint mono">{t("backup.hint")}</div>
  </div>

  <div class="group">
    <SettingRow label={t("backup.format")} hint={formatHint}>
      {#snippet control()}
        <Select
          options={formats}
          bind:value={format}
          disabled={exporting}
          size="sm"
        />
      {/snippet}
    </SettingRow>

    <SettingRow
      label={t("backup.includePlaylists")}
      hint={includePlaylists
        ? t("backup.includePlaylistsHint")
        : t("backup.includePlaylistsOff")}
    >
      {#snippet control()}
        <Switch
          on={includePlaylists}
          label={t("backup.includePlaylistsToggle")}
          onclick={() => (includePlaylists = !includePlaylists)}
        />
      {/snippet}
    </SettingRow>

    <SettingRow
      label={t("backup.contents")}
      hint={trackCount === null
        ? t("common.loading")
        : trackCount === 0
          ? t("backup.nothingToExport")
          : t("backup.contentsHint", {
              tracks: trackCount,
              playlists: playlistCount,
            })}
    >
      {#snippet control()}
        <Button variant="primary" size="sm" disabled={busy} onclick={runExport}>
          <Download size={13} stroke-width={1.5} />
          {exporting ? t("backup.exporting") : t("backup.export")}
        </Button>
      {/snippet}
    </SettingRow>
  </div>

  {#if lastExport}
    <div class="section">
      <div class="last mono">
        {lastExport.trackCount}
        {lastExport.playlistCount > 0 ? ` · ${lastExport.playlistCount}` : ""}
        · {fmtBytes(lastExport.bytes)} · {lastExport.format.toUpperCase()}
      </div>
      <div class="last-path mono">{lastExport.path}</div>
    </div>
  {/if}

  <div class="section restore">
    <div class="label mono">{t("backup.restoreTitle")}</div>
    <div class="hint mono">{t("backup.restoreHint")}</div>
    <div>
      <Button size="sm" disabled={importing} onclick={runImport}>
        <Upload size={13} stroke-width={1.5} />
        {importing ? t("backup.restoring") : t("backup.restore")}
      </Button>
    </div>
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
    box-shadow: var(--shadow-sm);
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

  .last {
    font-size: 11px;
    color: var(--dim);
    letter-spacing: 0.04em;
  }

  .last-path {
    font-size: 10.5px;
    color: var(--faint);
    line-height: 1.5;
    overflow-wrap: anywhere;
  }

  .restore {
    border-top: 1px solid var(--line);
    gap: 8px;
  }
</style>
