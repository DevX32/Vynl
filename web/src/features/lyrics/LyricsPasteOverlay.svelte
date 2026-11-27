<script lang="ts">
  import { t } from "@lib/i18n";
  import type { LyricsResult } from "@lib/types";
  import {
    lyricsKindOf,
    setLyricsManual,
    toLyricLines,
  } from "@state/lyrics.svelte";

  let { onClose }: { onClose: () => void } = $props();

  let text = $state("");

  function apply(): void {
    const value = text.trim();
    if (!value) return;
    const result: LyricsResult = {
      kind: lyricsKindOf(value),
      text: value,
      source: "local",
    };
    setLyricsManual(result, toLyricLines(result));
    onClose();
  }
</script>

<textarea
  class="paste-input mono"
  bind:value={text}
  placeholder={t("lyrics.pastePlaceholder")}
  rows={12}></textarea>
<div class="actions">
  <button class="btn" onclick={onClose}>{t("common.cancel")}</button>
  <button class="btn primary" onclick={apply} disabled={!text.trim()}>
    {t("lyrics.apply")}
  </button>
</div>

<style>
  .paste-input {
    width: 100%;
    min-height: 160px;
    resize: vertical;
    padding: 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line-strong);
    background: var(--bg-raise);
    color: var(--text);
    font-size: 11px;
    line-height: 1.5;
    outline: none;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn {
    appearance: none;
    border: 1px solid color-mix(in srgb, var(--text) 18%, transparent);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--dim);
    padding: 7px 16px;
    font-size: 10.5px;
    letter-spacing: 0.06em;
    cursor: pointer;
    transition:
      color 0.15s,
      border-color 0.15s,
      background 0.15s;
  }

  .btn:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--line-strong);
  }

  .btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .btn.primary {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 58%, transparent);
    background: var(--accent-soft);
  }

  .btn.primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    border-color: var(--accent);
  }
</style>
