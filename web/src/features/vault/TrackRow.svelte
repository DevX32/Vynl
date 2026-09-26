<script lang="ts">
  import type {
    MatchSource,
    SearchCandidate,
    TrackMeta,
    TrackProgress,
  } from "../../lib/types";
  import { DURATION_TOLERANCE } from "../../lib/constants";
  import { AlertTriangle } from "lucide-svelte";
  import { blobSrc } from "../../lib/format";
  import { t } from "@lib/i18n";
  import Select from "../../components/Select.svelte";

  let {
    track,
    progress,
    candidates,
    picked,
    allowPick,
    onpick,
  }: {
    track: TrackMeta;
    progress: TrackProgress | undefined;
    candidates?: SearchCandidate[];
    picked?: number;
    allowPick?: boolean;
    onpick?: (trackId: string, index: number) => void;
  } = $props();

  const SOURCE: Record<MatchSource, string> = {
    youtube: "YT",
    youtubeMusic: "YTM",
  };

  function statusLabel(s: TrackProgress["status"], message?: string): string {
    switch (s) {
      case "done": return t("trackRow.done");
      case "error": return t("trackRow.failed");
      case "skipped":
        if (message?.includes("length")) return t("trackRow.badLen");
        return t("trackRow.exists");
      case "cancelled": return t("trackRow.cancelled");
      case "searching": return t("trackRow.search");
      case "processing": return t("trackRow.tag");
      case "downloading": return "";
      default: return t("trackRow.queue");
    }
  }

  const status = $derived(progress?.status);
  const isDownloading = $derived(status === "downloading");
  const matchIndex = $derived(picked ?? 0);
  const selected = $derived(
    candidates && candidates.length > 0
      ? candidates[Math.min(matchIndex, candidates.length - 1)]
      : undefined,
  );
  const poorMatch = $derived(
    !!selected &&
      !!selected.duration &&
      !!track.duration &&
      Math.abs(selected.duration - track.duration) / track.duration > DURATION_TOLERANCE,
  );
</script>

<div
  class="row"
  class:done={status === "done"}
  class:failed={status === "error"}
  class:active={isDownloading || status === "processing" || status === "searching"}
>
  <span class="c-num mono" class:hidden={!track.trackNumber}>
    {track.trackNumber != null ? String(track.trackNumber).padStart(2, "0") : "··"}
  </span>

  <span class="c-cover">
    {#if track.cover}
      <img class="cover-img" use:blobSrc={track.cover} alt="" loading="lazy" />
    {:else}
      <div class="cover-placeholder"></div>
    {/if}
  </span>

  <div class="c-title">
    <div class="title">{track.title}</div>
    <div class="sub mono">
      {track.artist}{#if track.album} · {track.album}{/if}
    </div>
  </div>

  <div class="c-source">
    {#if allowPick && candidates && candidates.length > 0}
      <Select
        size="sm"
        options={candidates.map((c, i) => ({
          value: String(i),
          label: `${SOURCE[c.source]}${c.channel ? ` · ${c.channel}` : ""} — ${c.title}`,
        }))}
        value={String(matchIndex)}
        label={t("trackRow.sourceMatch")}
        onchange={(v) => onpick?.(track.id, Number(v))}
      />
      {#if poorMatch}
        <AlertTriangle class="warn" size={12} stroke-width={1.5} />
      {/if}
    {/if}
  </div>

  <div class="c-status">
    {#if progress}
      <span
        class="status-label mono {status}"
        title={status === "error" ? progress.message : undefined}
      >
        {statusLabel(progress.status, progress.message)}
      </span>
    {/if}
    {#if progress && (isDownloading || (progress.percent > 0 && progress.percent < 100))}
      <div class="c-progress">
        <div class="prog-bar" class:low={!isDownloading}>
          <div class="prog-fill" style:width={`${progress.percent}%`}></div>
        </div>
        <span class="prog-pct mono">{Math.round(progress.percent)}%</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: var(--track-cols, 32px 36px minmax(0, 1fr) minmax(140px, 1fr) 70px);
    align-items: center;
    gap: var(--col-gap, 8px);
    padding: var(--row-py, 8px) var(--row-px, 0px);
    transition: background 0.12s ease;
  }

  .row:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .row.active {
    background: color-mix(in srgb, var(--accent) 6%, transparent);
  }

  .row.done {
    opacity: 0.5;
  }

  .row.failed {
    background: color-mix(in srgb, var(--red) 4%, transparent);
  }

  .c-num {
    text-align: right;
    font-size: 11px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }

  .c-num.hidden {
    visibility: hidden;
  }

  .c-cover {
    width: var(--cover-w, 36px);
    height: var(--cover-w, 36px);
  }

  .cover-img {
    width: var(--cover-w, 36px);
    height: var(--cover-w, 36px);
    border-radius: var(--radius-sm);
    object-fit: cover;
    background: var(--bg-raise);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.15);
  }

  .cover-placeholder {
    width: var(--cover-w, 36px);
    height: var(--cover-w, 36px);
    border-radius: var(--radius-sm);
    background: var(--placeholder-gradient);
  }

  .c-title {
    min-width: 0;
  }

  .title {
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.3;
  }

  .sub {
    font-size: 11px;
    color: var(--dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.3;
  }

  .c-source {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .source-text {
    font-size: 11px;
    color: var(--faint);
  }

  .warn {
    font-size: 12px;
    color: var(--amber);
    cursor: help;
  }

  .c-status {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    justify-content: center;
    gap: 3px;
    text-align: right;
  }

  .status-label {
    font-size: 10.5px;
    letter-spacing: 0.06em;
    white-space: nowrap;
  }

  .status-label.done { color: var(--green); }
  .status-label.error { color: var(--red); }
  .status-label.skipped { color: var(--amber); }
  .status-label.cancelled,
  .status-label.queued { color: var(--faint); }
  .status-label.searching {
    color: var(--dim);
    animation: blink 1s steps(2) infinite;
  }

  .c-progress {
    display: flex;
    align-items: center;
    gap: 4px;
    justify-content: flex-end;
  }

  .prog-bar {
    width: 32px;
    height: 3px;
    background: var(--line-strong);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .prog-bar.low {
    background: var(--line);
  }

  .prog-fill {
    height: 100%;
    background: var(--accent);
    border-radius: var(--radius-sm);
    transition: width 0.2s linear;
  }

  .prog-pct {
    font-size: 11px;
    color: var(--accent);
    min-width: 24px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  @container player (max-width: 620px) {
    .c-source {
      display: none;
    }
  }

  @container player (max-width: 420px) {
    .c-num {
      display: none;
    }

    .title {
      font-size: 12px;
    }

    .sub {
      font-size: 10px;
    }
  }
</style>