<script lang="ts">
  import { RefreshCw } from "lucide-svelte";
  import { t } from "@lib/i18n";
  import { getNickname, rollNickname } from "@lib/nickname.svelte";
  import ColorPicker from "@components/ColorPicker.svelte";
  import Button from "@components/Button.svelte";
  import Switch from "@components/Switch.svelte";
  import { getCurrentSettings, patchSettings } from "@state/settings.svelte";
  import { ACCENT_PRESETS } from "@lib/color";
  import { openPluginsUi } from "@state/plugins.svelte";
  import MobileSyncSettings from "./MobileSyncSettings.svelte";
  import SettingRow from "./SettingRow.svelte";

  const settings = $derived(getCurrentSettings());

  let _displayNameTimer: ReturnType<typeof setTimeout> | null = null;
  let _displayNameInput: HTMLInputElement | undefined = $state();

  function debouncedDisplayNamePatch(value: string): void {
    if (_displayNameTimer) clearTimeout(_displayNameTimer);
    _displayNameTimer = setTimeout(() => {
      void patchSettings({ displayName: value });
      _displayNameTimer = null;
    }, 300);
  }

  function rollDisplayName(): void {
    if (_displayNameTimer) {
      clearTimeout(_displayNameTimer);
      _displayNameTimer = null;
    }
    const next = rollNickname();
    if (_displayNameInput) _displayNameInput.value = next;
    void patchSettings({ displayName: next });
  }
</script>

<div class="panel">
  <div class="section">
    <div class="label mono">{t("settings.displayName")}</div>
    <input
      class="text"
      bind:this={_displayNameInput}
      value={settings.displayName}
      oninput={(e) => debouncedDisplayNamePatch(e.currentTarget.value)}
      spellcheck={false}
      placeholder={getNickname()}
    />
    <div class="hint-row">
      <div class="hint mono">{t("settings.displayNameHint")}</div>
      <Button size="sm" onclick={rollDisplayName}>
        <RefreshCw size={13} stroke-width={1.5} />
        {t("settings.displayNameRoll")}
      </Button>
    </div>
  </div>

  <div class="group">
    <SettingRow
      label={t("plugins.title")}
      hint={t("plugins.openHint")}
    >
      {#snippet control()}
        <Button size="sm" onclick={openPluginsUi}>
          {t("plugins.open")}
        </Button>
      {/snippet}
    </SettingRow>

    <MobileSyncSettings />
  </div>

  <div class="section">
    <div class="label mono">{t("settings.accentColor")}</div>
    <ColorPicker
      value={settings.accentColor}
      presets={ACCENT_PRESETS}
      label={t("settings.accentColorCustom")}
      disabled={settings.dynamicAccent}
      onchange={(hex) => void patchSettings({ accentColor: hex })}
    />
    <div class="hint mono">{t("settings.accentColorHint")}</div>
  </div>

  <div class="group">
    <SettingRow
      label={t("settings.dynamicAccent")}
      hint={t("settings.dynamicAccentHint")}
    >
      {#snippet control()}
        <Switch
          on={settings.dynamicAccent}
          label={t("settings.dynamicAccentToggle")}
          onclick={() =>
            void patchSettings({ dynamicAccent: !settings.dynamicAccent })}
        />
      {/snippet}
    </SettingRow>

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
    gap: 8px;
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


</style>
