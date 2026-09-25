<script lang="ts">
  import { t } from "@lib/i18n";
  import SettingsPanel from "./SettingsPanel.svelte";
  import PluginSettings from "./PluginSettings.svelte";
  import ToolSetup from "./ToolSetup.svelte";
  import UpdateBanner from "./UpdateBanner.svelte";
  import SupportBanner from "./SupportBanner.svelte";
</script>

<section class="settings-page">
  <div class="head">
    <div class="kicker mono">VYNL</div>
    <div class="title display">{t("settings.title")}</div>
  </div>

  <div class="body">
    <UpdateBanner />
    <ToolSetup />
    <SettingsPanel />
    <PluginSettings />
    <SupportBanner />
  </div>
</section>

<style>
  .settings-page {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .head {
    flex-shrink: 0;
    padding: 20px 0 6px;
  }

  .kicker {
    font-size: 10px;
    color: var(--faint);
    letter-spacing: 0.2em;
    margin-bottom: 8px;
  }

  .title {
    font-size: 34px;
    line-height: 1;
  }

  .body {
    flex: 1;
    min-height: 0;
    padding-top: 16px;
    overflow-x: hidden;
    overflow-y: auto;
    scrollbar-width: none;
  }

  .body::-webkit-scrollbar {
    display: none;
  }
</style>
