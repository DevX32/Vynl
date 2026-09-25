<script lang="ts">
  import { t } from "@lib/i18n";
  import { vynl } from "../../lib/vynl";
  import { getCurrentTrack } from "@state/now-playing.svelte";
  import type { LyricsResult } from "../../lib/types";
  import { setLyricsManual, parseLrcText, toLyricLines } from "@state/lyrics.svelte";

  let {
    onClose,
  }: {
    onClose: () => void;
  } = $props();

  let pasteText = $state("");

  function applyPastedLyrics(): void {
    const text = pasteText.trim();
    if (!text) return;
    const isSynced = /\[\d{1,2}:\d{2}(?:[.:]\d{1,3})?\]/.test(text);
    const result: LyricsResult = { kind: isSynced ? "lrc" : "txt", text, source: "local" };
    setLyricsManual(result, isSynced ? parseLrcText(text) : toLyricLines(result));
    onClose();

    const track = getCurrentTrack();
    if (!track?.path) return;
    const plain = isSynced
      ? text.replace(/^\[\d{1,2}:\d{2}(?:[.:]\d{1,3})?\]\s*/gm, "").trim()
      : text;
    if (plain) void vynl.lyricsEmbed({ file: track.path, text: plain }).catch(() => {});
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
  <button class="btn-primary" onclick={applyPastedLyrics} disabled={!pasteText.trim()}>
    {t("lyrics.apply")}
  </button>
</div>

<style>
  .paste-input {
    background: var(--bg);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 10px;
    color: var(--text);
    font-size: 11px;
    line-height: 1.5;
    resize: vertical;
    outline: none;
    min-height: 160px;
    transition: border-color 0.15s;
  }
  .paste-input:focus {
    border-color: var(--accent);
  }
  .paste-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
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

  .btn-primary {
    background: var(--accent);
    border: none;
    border-radius: var(--radius-sm);
    color: var(--bg);
    padding: 7px 16px;
    font-size: 10.5px;
    letter-spacing: 0.06em;
    cursor: pointer;
    transition: opacity 0.15s, transform 0.1s;
  }
  .btn-primary:hover:not(:disabled) {
    opacity: 0.9;
  }
  .btn-primary:active:not(:disabled) {
    transform: scale(0.97);
  }
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
