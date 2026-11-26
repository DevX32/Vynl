import { t } from "@lib/i18n";
import { toasts } from "@lib/toast";
import { formatSleepRemaining } from "@lib/playback";

type SleepTimerMode = "off" | "time" | "end";

let _mode = $state<SleepTimerMode>("off");
let _remaining = $state(0);
let _minutes = $state(0);
let _deadline = 0;
let _ticker: ReturnType<typeof setInterval> | null = null;
let _pauseHandler: (() => void) | null = null;

export function setSleepPauseHandler(fn: (() => void) | null): () => void {
  _pauseHandler = fn;
  return () => {
    if (_pauseHandler === fn) _pauseHandler = null;
  };
}

function stopTicker(): void {
  if (_ticker !== null) {
    clearInterval(_ticker);
    _ticker = null;
  }
}

function expire(): void {
  const pause = _pauseHandler;
  stopTicker();
  _mode = "off";
  _deadline = 0;
  _remaining = 0;
  _minutes = 0;
  pause?.();
  toasts.info(t("player.sleepTimerDone"));
}

function tick(): void {
  const left = Math.max(0, _deadline - Date.now());
  _remaining = left;
  if (left <= 0) expire();
}

export function startSleepTimer(minutes: number): void {
  if (!Number.isFinite(minutes) || minutes <= 0) return;
  stopTicker();
  _mode = "time";
  _minutes = minutes;
  _deadline = Date.now() + minutes * 60_000;
  _remaining = minutes * 60_000;
  _ticker = setInterval(tick, 1000);
}

export function sleepAtEndOfTrack(): void {
  stopTicker();
  _mode = "end";
  _minutes = 0;
  _deadline = 0;
  _remaining = 0;
}

export function cancelSleepTimer(): void {
  stopTicker();
  _mode = "off";
  _minutes = 0;
  _deadline = 0;
  _remaining = 0;
}

export function getSleepMode(): SleepTimerMode {
  return _mode;
}

export function getSleepMinutes(): number {
  return _minutes;
}

export function getSleepRemainingLabel(): string {
  return formatSleepRemaining(_remaining);
}

export function consumeEndOfTrack(): boolean {
  if (_mode !== "end") return false;
  cancelSleepTimer();
  toasts.info(t("player.sleepAtEndDone"));
  return true;
}
