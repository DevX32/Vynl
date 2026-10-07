<script lang="ts">
  import { t } from "@lib/i18n";
  import { vynl } from "@lib/vynl";
  import { Coffee } from "lucide-svelte";
  import SettingRow from "./SettingRow.svelte";

  const KOFI_URL = "https://ko-fi.com/devx32";
</script>

<div class="group">
  <SettingRow label={t("settings.support")} hint={t("settings.supportHint")}>
    {#snippet control()}
      <button
        class="kofi-btn mono"
        onclick={() => void vynl.openExternal(KOFI_URL)}
      >
        <Coffee size={13} stroke-width={1.5} />
        {t("settings.supportButton")}
      </button>
    {/snippet}
  </SettingRow>
</div>

<style>
  .group {
    border-top: 1px solid var(--line);
  }

  .kofi-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    letter-spacing: 0.04em;
    color: var(--red);
    background: color-mix(in srgb, var(--red) 10%, transparent);
    border: none;
    border-radius: var(--radius-sm);
    padding: 8px 16px;
    white-space: nowrap;
    cursor: pointer;
    transition:
      background 0.15s,
      transform 0.1s;
  }

  .kofi-btn:hover {
    background: color-mix(in srgb, var(--red) 18%, transparent);
  }

  .kofi-btn:active {
    transform: scale(0.97);
  }
</style>