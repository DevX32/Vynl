<script lang="ts">
  import { t } from "@lib/i18n";
  import { vynl } from "@lib/vynl";
  import { Coffee } from "lucide-svelte";

  const KOFI_URL = "https://ko-fi.com/devx32";
</script>

<div class="support">
  <div class="icon">
    <Coffee size={18} />
  </div>
  <div class="info">
    <span class="title mono">{t("settings.support")}</span>
    <span class="hint mono">{t("settings.supportHint")}</span>
  </div>
  <button class="kofi-btn mono" onclick={() => void vynl.openExternal(KOFI_URL)}>
    {t("settings.supportButton")}
  </button>
</div>

<style>
  .support {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    background: var(--surface);
    margin-bottom: 18px;
  }

  .icon {
    flex-shrink: 0;
    width: 34px;
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    color: #ff5e5b;
    background: rgba(255, 94, 91, 0.12);
  }

  .info {
    flex: 1;
    min-width: 0;
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

  .kofi-btn {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    letter-spacing: 0.04em;
    color: #ff5e5b;
    background: rgba(255, 94, 91, 0.1);
    border-radius: var(--radius-sm);
    padding: 8px 16px;
    white-space: nowrap;
    transition:
      background 0.15s,
      border-color 0.15s,
      transform 0.1s;
  }

  .kofi-btn:hover {
    background: rgba(255, 94, 91, 0.18);
  }

  .kofi-btn:active {
    transform: scale(0.97);
  }
</style>