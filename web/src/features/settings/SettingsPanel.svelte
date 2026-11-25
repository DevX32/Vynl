<script lang="ts">
  import { onDestroy, onMount, untrack } from "svelte";
  import { RefreshCw } from "lucide-svelte";
  import { t } from "@lib/i18n";
  import { getNickname, rollNickname } from "@lib/nickname.svelte";
  import ColorPicker from "@components/ColorPicker.svelte";
  import ThemePicker from "@components/ThemePicker.svelte";
  import Button from "@components/Button.svelte";
  import Switch from "@components/Switch.svelte";
  import Dialog from "@components/Dialog.svelte";
  import { getCurrentSettings, patchSettings } from "@state/settings.svelte";
  import { ACCENT_PRESETS } from "@lib/color";
  import { vynl } from "@lib/vynl";
  import { toasts } from "@lib/toast";
  import { refreshLibrary } from "@state/library.svelte";
  import { refreshPlaylists } from "@state/playlists.svelte";
  import MobileSyncSettings from "./MobileSyncSettings.svelte";
  import SettingRow from "./SettingRow.svelte";
  import TasteSettings from "./TasteSettings.svelte";
  import Equalizer from "./Equalizer.svelte";
  import CrossfadeControl from "./CrossfadeControl.svelte";
  import { SPEED_STEPS, SLEEP_STEPS, formatSpeed } from "@lib/playback";
  import {
    getPlaybackSpeed,
    setPlaybackSpeed,
    getReplayGain,
    setReplayGain,
  } from "@state/player.svelte";
  import {
    cancelSleepTimer,
    getSleepMinutes,
    getSleepMode,
    getSleepRemainingLabel,
    sleepAtEndOfTrack,
    startSleepTimer,
  } from "@state/sleep-timer.svelte";

  const settings = $derived(getCurrentSettings());

  const playbackSpeed = $derived(getPlaybackSpeed());
  const replayGain = $derived(getReplayGain());
  const sleepMode = $derived(getSleepMode());
  const sleepMinutes = $derived(getSleepMinutes());

  let _clearingCache = $state(false);
  let _confirmingClear = $state(false);
  let _downloads = $state<{ files: number; bytes: number } | null>(null);
  let _downloadsLoading = $state(false);
  let _confirmingRemove = $state(false);
  let _removingDownloads = $state(false);

  function errText(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  function fmtBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }

  const downloadsHint = $derived.by(() => {
    if (_downloads === null) return t("settings.downloadedSongsReading");
    if (_downloads.files === 0) return t("settings.downloadedSongsNone");
    return t("settings.downloadedSongsHint", {
      count: _downloads.files,
      size: fmtBytes(_downloads.bytes),
    });
  });

  const canRemoveDownloads = $derived(
    (_downloads?.files ?? 0) > 0 && !_removingDownloads,
  );

  async function refreshDownloads(): Promise<void> {
    if (_downloadsLoading) return;
    _downloadsLoading = true;
    try {
      _downloads = await vynl.downloadsStats();
    } catch (error) {
      console.warn("[settings] failed to read downloads:", error);
      _downloads = null;
    } finally {
      _downloadsLoading = false;
    }
  }

  async function removeDownloads(): Promise<void> {
    _confirmingRemove = false;
    if (_removingDownloads) return;
    _removingDownloads = true;
    try {
      const result = await vynl.deleteAllLibraryTracks();
      await Promise.all([refreshLibrary(), refreshPlaylists(), refreshDownloads()]);
      if (result.failed > 0) {
        toasts.error(
          t("common.deletePartial", { failed: result.failed }),
        );
      } else {
        toasts.success(
          t("downloads.removeDownloadedDone", { deleted: result.deleted }),
        );
      }
    } catch (error) {
      toasts.error(
        t("downloads.removeDownloadedFailed", { error: errText(error) }),
      );
    } finally {
      _removingDownloads = false;
    }
  }

  onMount(() => {
    void refreshDownloads();
  });

  async function clearCache(): Promise<void> {
    _confirmingClear = false;
    if (_clearingCache) return;
    _clearingCache = true;
    try {
      const freed = await vynl.clearCache();
      const mb = Math.round(freed / (1024 * 1024));
      toasts.success(t("settings.cacheCleared", { mb }));
    } catch (error) {
      toasts.error(t("settings.cacheClearFailed", { error: errText(error) }));
    } finally {
      _clearingCache = false;
    }
  }

  let _displayNameTimer: ReturnType<typeof setTimeout> | null = null;
  let _displayNameInput: HTMLInputElement | undefined = $state();
  let _rolling = $state(false);
  let _rollTimer: ReturnType<typeof setTimeout> | null = null;

  const ROLL_LOCK_MS = 450;
  const _initialDisplayName = untrack(() => settings.displayName);

  function debouncedDisplayNamePatch(value: string): void {
    if (_displayNameTimer) clearTimeout(_displayNameTimer);
    _displayNameTimer = setTimeout(() => {
      void patchSettings({ displayName: value });
      _displayNameTimer = null;
    }, 300);
  }

  function rollDisplayName(): void {
    if (_rolling) return;
    if (_displayNameTimer) {
      clearTimeout(_displayNameTimer);
      _displayNameTimer = null;
    }
    const next = rollNickname();
    if (_displayNameInput) _displayNameInput.value = next;
    void patchSettings({ displayName: next });
    _rolling = true;
    _rollTimer = setTimeout(() => {
      _rolling = false;
      _rollTimer = null;
    }, ROLL_LOCK_MS);
  }

  function flushDebouncedDisplayName(): void {
    if (!_displayNameTimer) return;
    clearTimeout(_displayNameTimer);
    _displayNameTimer = null;
    const value = _displayNameInput?.value;
    if (value !== undefined && value !== getCurrentSettings().displayName) {
      void patchSettings({ displayName: value });
    }
  }

  onDestroy(() => {
    flushDebouncedDisplayName();
    if (_rollTimer) clearTimeout(_rollTimer);
  });
</script>

<div class="panel">
  <div class="group-head first">
    <span class="label mono">{t("settings.groupProfile")}</span>
  </div>

  <div class="section flush">
    <div class="label mono">{t("settings.nickname")}</div>
    <input
      class="text"
      bind:this={_displayNameInput}
      value={_initialDisplayName}
      oninput={(e) => debouncedDisplayNamePatch(e.currentTarget.value)}
      spellcheck={false}
      placeholder={getNickname()}
    />
    <div class="hint-row">
      <div class="hint mono">{t("settings.nicknameHint")}</div>
      <Button size="sm" onclick={rollDisplayName} disabled={_rolling}>
        <span class="roll-icon">
          <RefreshCw size={13} stroke-width={1.5} class={_rolling ? "spin" : ""} />
        </span>
        {t("settings.nicknameRoll")}
      </Button>
    </div>
  </div>

  <TasteSettings />

  <div class="group">
    <MobileSyncSettings />
  </div>

  <div class="group">
    <div class="group-head">
      <span class="label mono">{t("settings.groupAppearance")}</span>
    </div>
    <div class="section flush">
      <div class="label mono">{t("settings.theme")}</div>
      <ThemePicker
        value={settings.theme}
        accent={settings.accentColor}
        label={t("settings.themePicker")}
        onchange={(id) => void patchSettings({ theme: id })}
      />
      <div class="hint mono">{t("settings.themeHint")}</div>
    </div>

    <div class="section flush divider accent-section">
      <div class="label-row">
        <div class="label mono">{t("settings.accentColor")}</div>
      </div>
      <div class="label-toggle">
        <span class="label mono">{t("settings.dynamicAccent")}</span>
        <Switch
          on={settings.dynamicAccent}
          label={t("settings.dynamicAccentToggle")}
          onclick={() =>
            void patchSettings({ dynamicAccent: !settings.dynamicAccent })}
        />
      </div>
      <ColorPicker
        value={settings.accentColor}
        presets={ACCENT_PRESETS}
        label={t("settings.accentColorCustom")}
        disabled={settings.dynamicAccent}
        onchange={(hex) => void patchSettings({ accentColor: hex })}
      />
      <div class="hint mono">
        {settings.dynamicAccent
          ? t("settings.dynamicAccentHint")
          : t("settings.accentColorHint")}
      </div>
    </div>
  </div>

  <div class="group">
    <div class="group-head">
      <span class="label mono">{t("common.equalizer")}</span>
    </div>
    <div class="section flush">
      <Equalizer />
      <CrossfadeControl />
    </div>
  </div>

  <div class="group">
    <div class="section flush">
      <div class="label mono">{t("player.playbackSpeed")}</div>
      <div class="chips">
        {#each SPEED_STEPS as step (step)}
          <Button
            size="sm"
            selected={step === playbackSpeed}
            onclick={() => setPlaybackSpeed(step)}
          >
            {formatSpeed(step)}
          </Button>
        {/each}
      </div>
      <div class="hint mono">{t("player.playbackSpeedHint")}</div>
    </div>

    <div class="divider"></div>
    <div class="rows">
      <SettingRow
        label={t("player.replayGain")}
        hint={t("player.replayGainHint")}
      >
        {#snippet control()}
          <Switch
            on={replayGain}
            label={t("player.replayGainToggle")}
            onclick={() => setReplayGain(!replayGain)}
          />
        {/snippet}
      </SettingRow>
    </div>

    <div class="section flush divider">
      <div class="label mono">{t("player.sleepTimer")}</div>
      <div class="hint mono">{t("player.sleepTimerHint")}</div>
      {#if sleepMode === "time"}
        <div class="hint mono">
          {t("player.sleepRemaining", { time: getSleepRemainingLabel() })}
        </div>
      {:else if sleepMode === "end"}
        <div class="hint mono">{t("player.sleepEndOfTrack")}</div>
      {/if}
      <div class="chips">
        <Button
          size="sm"
          selected={sleepMode === "off"}
          onclick={cancelSleepTimer}
        >
          {t("common.off")}
        </Button>
        {#each SLEEP_STEPS as minutes (minutes)}
          <Button
            size="sm"
            selected={sleepMode === "time" && sleepMinutes === minutes}
            onclick={() => startSleepTimer(minutes)}
          >
            {t("player.sleepMinutes", { n: minutes })}
          </Button>
        {/each}
        <Button
          size="sm"
          selected={sleepMode === "end"}
          onclick={sleepAtEndOfTrack}
        >
          {t("player.sleepEndOfTrack")}
        </Button>
      </div>
    </div>
  </div>

  <div class="group">
    <div class="group-head">
      <span class="label mono">{t("common.downloads")}</span>
    </div>
    <div class="rows">
      <SettingRow
        label={t("settings.overwrite")}
        hint={t("settings.overwriteHint")}
      >
        {#snippet control()}
          <Switch
            on={settings.overwrite}
            label={t("settings.overwriteToggle")}
            onclick={() =>
              void patchSettings({ overwrite: !settings.overwrite })}
          />
        {/snippet}
      </SettingRow>

      <SettingRow
        label={t("settings.confirmMatches")}
        hint={t("settings.confirmMatchesHint")}
      >
        {#snippet control()}
          <Switch
            on={settings.confirmMatches}
            label={t("settings.confirmMatchesToggle")}
            onclick={() =>
              void patchSettings({ confirmMatches: !settings.confirmMatches })}
          />
        {/snippet}
      </SettingRow>
    </div>
  </div>

  <div class="group">
    <div class="group-head">
      <span class="label mono">{t("settings.groupSystem")}</span>
    </div>
    <div class="rows">
      <SettingRow
        label={t("settings.minimizeToTray")}
        hint={t("settings.minimizeToTrayHint")}
      >
        {#snippet control()}
          <Switch
            on={settings.minimizeToTray}
            label={t("settings.minimizeToTrayToggle")}
            onclick={() =>
              void patchSettings({ minimizeToTray: !settings.minimizeToTray })}
          />
        {/snippet}
      </SettingRow>

      <SettingRow
        label={t("settings.hardwareAcceleration")}
        hint={t("settings.hardwareAccelerationHint")}
      >
        {#snippet control()}
          <Switch
            on={settings.hardwareAcceleration}
            label={t("settings.hardwareAccelerationToggle")}
            onclick={() =>
              void patchSettings({
                hardwareAcceleration: !settings.hardwareAcceleration,
              })}
          />
        {/snippet}
      </SettingRow>

      <SettingRow
        label={t("settings.launchAtStartup")}
        hint={t("settings.launchAtStartupHint")}
      >
        {#snippet control()}
          <Switch
            on={settings.launchAtStartup}
            label={t("settings.launchAtStartupToggle")}
            onclick={() =>
              void patchSettings({ launchAtStartup: !settings.launchAtStartup })}
          />
        {/snippet}
      </SettingRow>
    </div>
  </div>

  <div class="group">
    <div class="group-head">
      <span class="label mono">{t("settings.groupIntegrations")}</span>
    </div>
    <div class="rows">
      <SettingRow
        label={t("settings.discord")}
        hint={t("settings.discordHint")}
      >
        {#snippet control()}
          <Switch
            on={settings.discordRpc}
            label={t("settings.discordToggle")}
            onclick={() =>
              void patchSettings({ discordRpc: !settings.discordRpc })}
           />
         {/snippet}
       </SettingRow>

       <SettingRow
         label={t("settings.globalMediaKeys")}
         hint={t("settings.globalMediaKeysHint")}
       >
         {#snippet control()}
           <Switch
             on={settings.globalMediaKeys}
             label={t("settings.globalMediaKeysToggle")}
             onclick={() =>
               void patchSettings({ globalMediaKeys: !settings.globalMediaKeys })}
           />
         {/snippet}
       </SettingRow>
     </div>
   </div>
  <div class="group">
    <div class="group-head">
      <span class="label mono">{t("settings.groupStorage")}</span>
    </div>
    <div class="rows">
      <SettingRow
        label={t("settings.downloadedSongs")}
        hint={downloadsHint}
      >
        {#snippet control()}
          <Button
            size="sm"
            disabled={!canRemoveDownloads}
            onclick={() => (_confirmingRemove = true)}
          >
            {_removingDownloads
              ? t("downloads.removing")
              : t("common.delete")}
          </Button>
        {/snippet}
      </SettingRow>

      <SettingRow
        label={t("settings.clearCache")}
        hint={t("settings.clearCacheHint")}
      >
        {#snippet control()}
          <Button size="sm" disabled={_clearingCache} onclick={() => (_confirmingClear = true)}>
            {_clearingCache
              ? t("settings.cacheClearing")
              : t("common.clear")}
          </Button>
        {/snippet}
      </SettingRow>
    </div>
  </div>
</div>

{#if _confirmingClear}
  <Dialog
    open
    title={t("settings.clearCacheTitle")}
    description={t("settings.clearCacheBody")}
    confirmLabel={t("common.clear")}
    danger
    onconfirm={() => void clearCache()}
    oncancel={() => (_confirmingClear = false)}
  />
{/if}

{#if _confirmingRemove}
  <Dialog
    open
    title={t("downloads.removeAllTitle")}
    description={t("downloads.removeAllBody")}
    confirmLabel={t("downloads.removeAllConfirm")}
    danger
    onconfirm={() => void removeDownloads()}
    oncancel={() => (_confirmingRemove = false)}
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
    gap: 8px;
  }

  .section.flush {
    padding-top: 10px;
  }

  .group {
    border-top: 1px solid var(--line);
  }

  .group-head {
    padding: 14px 16px 0;
    display: flex;
    align-items: center;
  }

  .panel > .group-head.first {
    padding-top: 16px;
  }

  .divider {
    border-top: 1px solid var(--line);
  }

  .rows > :global(.row + .row) {
    border-top: 1px solid var(--line);
  }

  .label {
    font-size: 11px;
    color: var(--dim);
    letter-spacing: 0.1em;
  }

  .accent-section {
    position: relative;
  }

  .label-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding-right: 96px;
  }

  .label-toggle {
    position: absolute;
    top: 10px;
    right: 16px;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 15px;
  }

  .hint {
    font-size: 11px;
    color: var(--faint);
    line-height: 1.5;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .hint-row {
    display: flex;
    align-items: center;
    gap: 12px;
    justify-content: space-between;
  }

  .text {
    background: var(--bg-raise);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 10px 14px;
    outline: none;
    width: 100%;
    font-size: 12.5px;
    transition: border-color 0.15s;
  }

  .roll-icon {
    display: inline-flex;
    align-items: center;
  }

  .hint-row :global(button:disabled) {
    opacity: 0.75;
  }

  .roll-icon :global(.spin) {
    animation: roll-spin 0.7s linear infinite;
  }

  @keyframes roll-spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .roll-icon :global(.spin) {
      animation-duration: 2.4s;
    }
  }
</style>
