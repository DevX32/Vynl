import { getCurrentTime } from "./now-playing.svelte";

const DRIFT_TOLERANCE_SECS = 0.4;

let anchorMedia = 0;
let anchorPerf = 0;
let running = false;
let subs = 0;
let raf = 0;
let tick = $state(0);

function predictedNow(): number {
  const perf = performance.now();
  const predicted = anchorMedia + (perf - anchorPerf) / 1000;
  if (Math.abs(getCurrentTime() - predicted) > DRIFT_TOLERANCE_SECS) {
    anchorMedia = getCurrentTime();
    anchorPerf = perf;
  }
  return anchorMedia + (perf - anchorPerf) / 1000;
}

function frame(): void {
  tick = performance.now();
  raf = requestAnimationFrame(frame);
}

export function startClock(): void {
  subs += 1;
  if (running) return;
  running = true;
  anchorMedia = getCurrentTime();
  anchorPerf = performance.now();
  tick = anchorPerf;
  raf = requestAnimationFrame(frame);
}

export function stopClock(): void {
  if (subs > 0) subs -= 1;
  if (subs > 0 || !running) return;
  running = false;
  if (raf) cancelAnimationFrame(raf);
  raf = 0;
}

export function getSmoothTime(): number {
  // Read reactive tick so consumers re-render while the clock runs.
  void tick;
  return running ? predictedNow() : getCurrentTime();
}