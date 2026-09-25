<script lang="ts">
  import { Download, X } from "lucide-svelte";
  import { t } from "@lib/i18n";
  import { fmtDuration } from "../../lib/format";
  import type { DownloadSummary } from "../../lib/types";

  let {
    collection,
    running,
    reviewing,
    summary,
    counts,
    totalDuration,
    onBeginDownload,
    onConfirmDownload,
    onRetryFailed,
    onCancel,
    onDone,
  }: {
    collection: { tracks: unknown[] } | null;
    running: boolean;
    reviewing: boolean;
    summary: DownloadSummary | null;
    counts: { done: number; skipped: number; failed: number; queued: number; active: number };
    totalDuration: number;
    onBeginDownload: () => void;
    onConfirmDownload: () => void;
    onRetryFailed: () => void;
    onCancel: () => void;
    onDone: () => void;
  } = $props();

  const finishedOk = $derived(
    summary != null && !summary.cancelled && counts.failed === 0,
  );
</script>

<footer>
  <div class="footer-bar">
    <div class="controls">
      {#if running}
        {@const total = collection?.tracks.length ?? 0}
        {@const completed = counts.done + counts.failed + counts.skipped}
        <div class="progress-pill mono">
          <div class="pill-bar">
            <div
              class="pill-fill"
              style:width={`${total > 0 ? (completed / total) * 100 : 0}%`}
            ></div>
          </div>
          <span class="pill-text">{completed}/{total}</span>
          <span class="pill-dot {counts.failed > 0 ? 'has-fail' : ''}"></span>
        </div>
        <button class="download-btn abort" onclick={onCancel}>
          <X size={14} stroke-width={2} />
          <span>{t("vault.abort")}</span>
        </button>
      {:else if collection}
        {@const total = collection.tracks.length}
        {#if total > 0}
          <div class="track-pill mono">
            <span class="pill-num">{total}</span>
            <span class="pill-label"
              >{total === 1 ? t("vault.track") : t("vault.tracks")}</span
            >
            <span class="pill-sep"></span>
            <span class="pill-dur">{fmtDuration(totalDuration)}</span>
          </div>
        {/if}
        <button
          class="download-btn"
          class:done={finishedOk}
          onclick={finishedOk
            ? onDone
            : reviewing
              ? onConfirmDownload
              : counts.failed > 0
                ? onRetryFailed
                : onBeginDownload}
        >
          <Download size={14} stroke-width={2} />
          {#if reviewing}
            <span>{t("vault.start")}</span>
          {:else if counts.failed > 0}
            <span>{t("vault.retryFailed", { n: counts.failed })}</span>
          {:else if finishedOk}
            <span>{t("vault.doneButton")}</span>
          {:else}
            <span>{t("vault.download")}</span>
          {/if}
        </button>
      {/if}
    </div>
  </div>
</footer>

<style>
  footer {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px;
    margin-bottom: -6px;
  }

  .footer-bar {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 14px;
  }

  .controls {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .progress-pill {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    height: 36px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line);
  }

  .pill-bar {
    width: 48px;
    height: 3px;
    background: var(--line-strong);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .pill-fill {
    height: 100%;
    background: var(--accent);
    border-radius: var(--radius-sm);
    transition: width 0.25s linear;
  }

  .pill-text {
    font-size: 11px;
    letter-spacing: 0.04em;
    color: var(--dim);
  }

  .pill-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--green);
    flex-shrink: 0;
  }

  .pill-dot.has-fail {
    background: var(--red);
  }

  .track-pill {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 10px;
    height: 36px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line);
    font-size: 11px;
    letter-spacing: 0.04em;
  }

  .pill-num {
    color: var(--text);
    font-weight: 500;
  }

  .pill-label {
    color: var(--faint);
  }

  .pill-sep {
    width: 1px;
    height: 12px;
    background: var(--line-strong);
    margin: 0 3px;
  }

  .pill-dur {
    color: var(--dim);
  }

  .download-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 0 18px;
    height: 36px;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.04em;
    color: var(--accent);
    background: var(--accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
    cursor: pointer;
    white-space: nowrap;
    transition:
      color 0.15s,
      background 0.15s,
      border-color 0.15s,
      transform 0.1s;
  }

  .download-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
  }

  .download-btn:active:not(:disabled) {
    transform: scale(0.97);
  }

  .download-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .download-btn.done {
    background: transparent;
    color: var(--green);
    border-color: color-mix(in srgb, var(--green) 35%, transparent);
  }

  .download-btn.done:hover {
    background: color-mix(in srgb, var(--green) 10%, transparent);
    border-color: color-mix(in srgb, var(--green) 50%, transparent);
  }

  .download-btn.abort {
    color: var(--red);
    background: rgba(255, 107, 97, 0.08);
    border-color: rgba(255, 107, 97, 0.35);
  }

  .download-btn.abort:hover {
    background: rgba(255, 107, 97, 0.15);
    border-color: var(--red);
  }
</style>
