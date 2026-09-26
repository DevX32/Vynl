<script lang="ts">
  import { t } from "@lib/i18n";
  import ColorPicker from "@components/ColorPicker.svelte";
  import { getCurrentSettings, patchSettings } from "@state/settings.svelte";
  import { ACCENT_PRESETS } from "@lib/color";
  import { openEq } from "@state/equalizer.svelte";
  import { getInstalledPlugins, openPluginsUi } from "@state/plugins.svelte";

  const settings = $derived(getCurrentSettings());
  const installedCount = $derived(getInstalledPlugins().length);

  let _displayNameTimer: ReturnType<typeof setTimeout> | null = null;
  function debouncedDisplayNamePatch(value: string): void {
    if (_displayNameTimer) clearTimeout(_displayNameTimer);
    _displayNameTimer = setTimeout(() => {
      void patchSettings({ displayName: value });
      _displayNameTimer = null;
    }, 300);
  }
</script>

<div class="panel">
  <div class="section">
    <div class="label mono">{t("settings.displayName")}</div>
    <input
      class="text"
      value={settings.displayName}
      oninput={(e) => debouncedDisplayNamePatch(e.currentTarget.value)}
      spellcheck={false}
      placeholder={t("settings.displayNamePlaceholder")}
    />
    <div class="hint mono">
      {t("settings.displayNameHint")}
    </div>
  </div>

  <div class="section row-section">
    <div>
      <div class="label mono">{t("settings.equalizer")}</div>
      <div class="hint mono">
        {settings.eqEnabled
          ? t("settings.equalizerHint")
          : t("settings.equalizerHintOff")}
      </div>
    </div>
    <div class="eq-actions">
      <span class="eq-status mono" class:on={settings.eqEnabled}>
        {settings.eqEnabled ? t("settings.stateOn") : t("settings.stateOff")}
      </span>
      <button class="chip mono" onclick={openEq}>
        {t("settings.equalizerOpen")}
      </button>
    </div>
  </div>

  <div class="section row-section">
    <div>
      <div class="label mono">{t("plugins.title")}</div>
      <div class="hint mono">{t("plugins.openHint")}</div>
    </div>
    <div class="eq-actions">
      <span class="eq-status mono on">
        {t("plugins.installedCount", { n: installedCount })}
      </span>
      <button class="chip mono" onclick={openPluginsUi}>
        {t("plugins.open")}
      </button>
    </div>
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
    <div class="hint mono">
      {t("settings.accentColorHint")}
    </div>
  </div>

  <div class="section row-section">
    <div>
      <div class="label mono">{t("settings.dynamicAccent")}</div>
      <div class="hint mono">
        {t("settings.dynamicAccentHint")}
      </div>
    </div>
    <button
      class="switch"
      class:on={settings.dynamicAccent}
      onclick={() =>
        void patchSettings({ dynamicAccent: !settings.dynamicAccent })}
      role="switch"
      aria-checked={settings.dynamicAccent}
      aria-label={t("settings.dynamicAccentToggle")}
    >
      <span class="knob"></span>
    </button>
  </div>

  <div class="section row-section">
    <div>
      <div class="label mono">{t("settings.overwrite")}</div>
      <div class="hint mono">
        {t("settings.overwriteHint")}
      </div>
    </div>
    <button
      class="switch"
      class:on={settings.overwrite}
      onclick={() => void patchSettings({ overwrite: !settings.overwrite })}
      role="switch"
      aria-checked={settings.overwrite}
      aria-label={t("settings.overwriteToggle")}
    >
      <span class="knob"></span>
    </button>
  </div>

  <div class="section row-section">
    <div>
      <div class="label mono">{t("settings.discord")}</div>
      <div class="hint mono">
        {t("settings.discordHint")}
      </div>
    </div>
    <button
      class="switch"
      class:on={settings.discordRpc}
      onclick={() => void patchSettings({ discordRpc: !settings.discordRpc })}
      role="switch"
      aria-checked={settings.discordRpc}
      aria-label={t("settings.discordToggle")}
    >
      <span class="knob"></span>
    </button>
  </div>

  <div class="section row-section">
    <div>
      <div class="label mono">{t("settings.confirmMatches")}</div>
      <div class="hint mono">
        {t("settings.confirmMatchesHint")}
      </div>
    </div>
    <button
      class="switch"
      class:on={settings.confirmMatches}
      onclick={() =>
        void patchSettings({ confirmMatches: !settings.confirmMatches })}
      role="switch"
      aria-checked={settings.confirmMatches}
      aria-label={t("settings.confirmMatchesToggle")}
    >
      <span class="knob"></span>
    </button>
  </div>

  <div class="section row-section">
    <div>
      <div class="label mono">{t("settings.minimizeToTray")}</div>
      <div class="hint mono">
        {t("settings.minimizeToTrayHint")}
      </div>
    </div>
    <button
      class="switch"
      class:on={settings.minimizeToTray}
      onclick={() =>
        void patchSettings({ minimizeToTray: !settings.minimizeToTray })}
      role="switch"
      aria-checked={settings.minimizeToTray}
      aria-label={t("settings.minimizeToTrayToggle")}
    >
      <span class="knob"></span>
    </button>
  </div>

  <div class="section row-section">
    <div>
      <div class="label mono">{t("settings.hardwareAcceleration")}</div>
      <div class="hint mono">
        {t("settings.hardwareAccelerationHint")}
      </div>
    </div>
    <button
      class="switch"
      class:on={settings.hardwareAcceleration}
      onclick={() =>
        void patchSettings({
          hardwareAcceleration: !settings.hardwareAcceleration,
        })}
      role="switch"
      aria-checked={settings.hardwareAcceleration}
      aria-label={t("settings.hardwareAccelerationToggle")}
    >
      <span class="knob"></span>
    </button>
  </div>

  <div class="section row-section">
    <div>
      <div class="label mono">{t("settings.launchAtStartup")}</div>
      <div class="hint mono">
        {t("settings.launchAtStartupHint")}
      </div>
    </div>
    <button
      class="switch"
      class:on={settings.launchAtStartup}
      onclick={() =>
        void patchSettings({ launchAtStartup: !settings.launchAtStartup })}
      role="switch"
      aria-checked={settings.launchAtStartup}
      aria-label={t("settings.launchAtStartupToggle")}
    >
      <span class="knob"></span>
    </button>
  </div>

</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
  }

  .section {
    padding: 16px;
    border-top: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .row-section {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
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

  .chip {
    font-size: 11px;
    color: var(--dim);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 4px 10px;
    transition:
      color 0.15s,
      background 0.15s;
  }

  .chip:hover {
    color: var(--text);
    border-color: var(--line-strong);
    background: var(--bg-raise);
  }

  .eq-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .eq-status {
    font-size: 10.5px;
    color: var(--faint);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    transition: color 0.15s;
  }

  .eq-status.on {
    color: var(--accent);
  }
</style>