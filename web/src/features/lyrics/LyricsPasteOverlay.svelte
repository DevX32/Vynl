<script lang="ts">
  import { t } from "@lib/i18n";
  import type { LyricsResult } from "../../lib/types";
  import { setLyricsManual, toLyricLines, lyricsKindOf } from "@state/lyrics.svelte";

  let {
    onClose,
  }: {
    onClose: () => void;
  } = $props();

  let pasteText = $state("");

  function applyPastedLyrics(): void {
    const text = pasteText.trim();
    if (!text) return;
    const result: LyricsResult = {
      kind: lyricsKindOf(text),
      text,
      source: "local",
    };
    setLyricsManual(result, toLyricLines(result));
    onClose();
  }
</script>

<textarea
  class="paste-input mono"
  bind:value={pasteText}
  placeholder={t("lyrics.pastePlaceholder")}
  rows={12}
></textarea>
<div class="paste-actions">
  <button class="btn-ghost" onclick={onClose}>{t("lyrics.cancel")}</button>
  <button class="btn-ghost btn-primary" onclick={applyPastedLyrics} disabled={!pasteText.trim()}>
    {t("lyrics.apply")}
  </button>
</div>

<style>
  .paste-input {
    background: var(--bg-raise);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 10px;
    color: var(--text);
    font-size: 11px;
    line-height: 1.5;
    resize: vertical;
    outline: none;
    min-height: 160px;
  }
  .paste-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn-ghost {
    appearance: none;
    -webkit-appearance: none;
    background: none;
    border: 1px solid color-mix(in srgb, var(--text) 18%, transparent);
    border-radius: var(--radius-sm);
    color: var(--dim);
    box-shadow: none;
    text-shadow: none;
    padding: 7px 16px;
    font-size: 10.5px;
    letter-spacing: 0.06em;
    cursor: pointer;
    transition:
      color 0.15s,
      border-color 0.15s,
      opacity 0.15s;
  }
  .btn-ghost:disabled {
    opacity: 0.62;
    cursor: not-allowed;
  }

  .btn-primary {
    color: var(--text);
    border-color: color-mix(in srgb, var(--accent) 70%, transparent);
    box-shadow: none;
  }
</style>
