import { getCurrentTime } from "./now-playing.svelte";

const DRIFT_TOLERANCE_SECS = 0.4;

let _anchorMedia = 0;
let _anchorPerf = 0;
let _running = false;
let _subs = 0;
let _raf = 0;

function predictedNow(): number {
  const perf = performance.now();
  const predicted = _anchorMedia + (perf - _anchorPerf) / 1000;

  if (Math.abs(getCurrentTime() - predicted) > DRIFT_TOLERANCE_SECS) {
    _anchorMedia = getCurrentTime();
    _anchorPerf = perf;
  }

  return _anchorMedia + (perf - _anchorPerf) / 1000;
}

function frame(): void {
  predictedNow();
  _raf = requestAnimationFrame(frame);
}

export function startClock(): void {
  _subs += 1;
  if (_running) return;
  _running = true;
  _anchorMedia = getCurrentTime();
  _anchorPerf = performance.now();
  _raf = requestAnimationFrame(frame);
}

export function stopClock(): void {
  if (_subs > 0) _subs -= 1;
  if (_subs > 0 || !_running) return;
  _running = false;
  if (_raf) cancelAnimationFrame(_raf);
  _raf = 0;
}

export function getSmoothTime(): number {
  return _running ? predictedNow() : getCurrentTime();
}