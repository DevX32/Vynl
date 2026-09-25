<script lang="ts">
  import { t } from "@lib/i18n";

  let {
    onClose,
  }: {
    onClose: () => void;
  } = $props();
</script>

<div class="help-items">
  <div class="help-item">
    <span class="help-key mono">{t("lyrics.helpKeyClick")}</span>
    {t("lyrics.helpClick")}
  </div>
  <div class="help-item">
    <span class="help-key mono">{t("lyrics.helpKeyCtrlClick")}</span>
    {t("lyrics.helpCtrlClick")}
  </div>
  <div class="help-item">
    <span class="help-key mono">{t("lyrics.helpKeyRightClick")}</span>
    {t("lyrics.helpRightClick")}
  </div>
  <div class="help-item">
    <span class="help-key mono">{t("lyrics.helpKeyEsc")}</span>
    {t("lyrics.helpEsc")}
  </div>
</div>
<button class="btn-ghost help-close" onclick={onClose}>{t("lyrics.gotIt")}</button>

<style>
  .help-items {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .help-item {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 12px;
    color: var(--dim);
  }
  .help-key {
    min-width: 92px;
    padding: 3px 8px;
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    font-size: 10px;
    letter-spacing: 0.06em;
    color: var(--text);
    text-align: center;
  }
  .help-close {
    align-self: flex-end;
    padding: 4px 2px;
  }

  .btn-ghost {
    background: none;
    border: none;
    color: var(--dim);
    font-size: 11px;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: color 0.15s;
  }
  .btn-ghost:hover {
    color: var(--accent);
  }
</style>
