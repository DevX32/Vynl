<script lang="ts">
  import { toasts } from "../lib/toast";
  import { AlertCircle, AlertTriangle, CheckCircle2, Info, X } from "lucide-svelte";
  import { fade, fly } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { t } from "@lib/i18n";

  const tone: Record<string, string> = {
    success: "var(--green)",
    error: "var(--red)",
    warning: "var(--amber)",
    info: "var(--accent)",
  };

  const icons: Record<string, typeof Info> = {
    success: CheckCircle2,
    error: AlertCircle,
    warning: AlertTriangle,
    info: Info,
  };

  const calm =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  const unit = (pct: number) => Math.min(1, Math.max(0, pct / 100));
</script>

<div class="stack" aria-live="polite" aria-atomic="false">
  {#each $toasts as toast (toast.id)}
    {@const Icon = icons[toast.kind] ?? Info}
    <div
      class="toast"
      class:paused={toast.paused}
      onpointerenter={() => toasts.pause(toast.id)}
      onpointerleave={() => toasts.resume(toast.id)}
      style:--tone={tone[toast.kind] ?? tone.info}
      role={toast.kind === "error" ? "alert" : undefined}
      animate:flip={{ duration: calm ? 0 : 160 }}
      in:fly={{ y: -8, duration: calm ? 0 : 200 }}
      out:fade={{ duration: calm ? 0 : 120 }}
    >
      <span class="icon" aria-hidden="true">
        <Icon size={16} strokeWidth={1.75} />
      </span>

      <p class="msg mono" title={toast.message}>{toast.message}</p>

      <button
        type="button"
        onclick={() => toasts.dismiss(toast.id)}
        aria-label={t("toast.dismiss")}
      >
        <X size={14} strokeWidth={1.5} />
      </button>

      <span class="timer" aria-hidden="true" style:transform={`scaleX(${unit(toast.progress)})`}></span>
    </div>
  {/each}
</div>

<style>
  .stack {
    position: fixed;
    top: calc(env(safe-area-inset-top, 0px) + 40px);
    left: 0;
    right: 0;
    margin-inline: auto;
    width: min(280px, calc(100vw - 32px));
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 6px;
    pointer-events: none;
  }

  .toast {
    --fg: var(--text, var(--dim));

    pointer-events: auto;
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 8px 10px 12px;
    overflow: hidden;
    background: var(--bg-raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
  }

  .icon {
    flex-shrink: 0;
    display: grid;
    place-items: center;
    height: 18px;
    color: var(--tone);
  }

  .msg {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 12.5px;
    line-height: 18px;
    color: var(--fg);
    overflow-wrap: anywhere;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    overflow: hidden;
  }

  button {
    flex-shrink: 0;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    margin-block: -3px;
    padding: 0;
    color: var(--faint);
    background: none;
    border: 0;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: color 120ms, background-color 120ms;
  }

  button:hover {
    color: var(--fg);
    background: var(--line);
  }

  button:focus-visible {
    color: var(--fg);
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .timer {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 2px;
    background: var(--tone);
    opacity: 0.6;
    transform-origin: left;
    transition: transform 30ms linear;
  }

  .paused .timer {
    opacity: 0.25;
  }

  @media (prefers-reduced-motion: reduce) {
    .timer,
    button {
      transition: none;
    }
  }
</style>