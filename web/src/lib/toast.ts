import { get, writable } from "svelte/store";

type ToastKind = "info" | "success" | "error" | "warning";

interface ToastItem {
  id: number;
  message: string;
  kind: ToastKind;
  progress: number;
  paused: boolean;
}

let nextId = 0;
const timers = new Map<number, ReturnType<typeof setInterval>>();
const items = writable<ToastItem[]>([]);

let audioCtx: AudioContext | null = null;
let audioCtxTimer: ReturnType<typeof setTimeout> | null = null;

const NOTIFY_PARTIALS = [
  { ratio: 1, level: 0.72, decay: 1 },
  { ratio: 2.003, level: 0.18, decay: 0.48 },
  { ratio: 3.01, level: 0.06, decay: 0.25 },
] as const;

function getAudioCtx(): AudioContext {
  if (!audioCtx) {
    audioCtx = new AudioContext();
  }
  if (audioCtx.state === "suspended") {
    void audioCtx.resume().catch(() => {});
  }
  if (audioCtxTimer) clearTimeout(audioCtxTimer);
  audioCtxTimer = setTimeout(() => {
    audioCtx?.close().catch(() => {});
    audioCtx = null;
    audioCtxTimer = null;
  }, 5000);
  return audioCtx;
}

function playNotifySound(): void {
  try {
    const ctx = getAudioCtx();
    const now = ctx.currentTime;

    const filter = ctx.createBiquadFilter();
    filter.type = "lowpass";
    filter.frequency.setValueAtTime(2800, now);
    filter.frequency.exponentialRampToValueAtTime(1600, now + 0.36);
    filter.Q.setValueAtTime(0.45, now);
    filter.connect(ctx.destination);

    const panners: StereoPannerNode[] = [];

    const playNote = (
      frequency: number,
      start: number,
      pan: number,
      duration: number,
    ): void => {
      let output: AudioNode = filter;
      if (typeof ctx.createStereoPanner === "function") {
        const panner = ctx.createStereoPanner();
        panner.pan.setValueAtTime(pan, start);
        panner.connect(filter);
        panners.push(panner);
        output = panner;
      }

      for (const partial of NOTIFY_PARTIALS) {
        const oscillator = ctx.createOscillator();
        const gain = ctx.createGain();
        const partialFrequency = frequency * partial.ratio;
        const decay = duration * partial.decay;
        oscillator.type = "sine";
        oscillator.frequency.setValueAtTime(
          partialFrequency * 1.002,
          start,
        );
        oscillator.frequency.exponentialRampToValueAtTime(
          partialFrequency,
          start + Math.min(0.04, decay),
        );
        gain.gain.setValueAtTime(0.0001, start);
        gain.gain.exponentialRampToValueAtTime(
          0.065 * partial.level,
          start + 0.014,
        );
        gain.gain.exponentialRampToValueAtTime(0.0001, start + decay);
        oscillator.connect(gain);
        gain.connect(output);
        oscillator.onended = () => {
          oscillator.disconnect();
          gain.disconnect();
        };
        oscillator.start(start);
        oscillator.stop(start + decay + 0.02);
      }
    };

    playNote(523.25, now, -0.18, 0.3);
    playNote(392, now + 0.085, 0, 0.27);
    playNote(523.25, now + 0.17, 0.18, 0.3);

    setTimeout(() => {
      for (const panner of panners) {
        try {
          panner.disconnect();
        } catch {}
      }
      try {
        filter.disconnect();
      } catch {}
    }, 700);
  } catch {}
}

function dismiss(id: number) {
  clearInterval(timers.get(id));
  timers.delete(id);
  items.update((l) => l.filter((t) => t.id !== id));
}

function createDismissTimer(id: number, duration: number): void {
  let remaining = duration;
  let last = Date.now();

  timers.set(
    id,
    setInterval(() => {
      const now = Date.now();
      const paused = get(items).find((toast) => toast.id === id)?.paused;
      if (paused) {
        last = now;
        return;
      }
      remaining -= now - last;
      last = now;
      const pct = Math.max(0, (remaining / duration) * 100);
      items.update((l) =>
        l.map((x) => (x.id === id ? { ...x, progress: pct } : x)),
      );
      if (pct <= 0) dismiss(id);
    }, 30),
  );
}

function push(message: string, kind: ToastKind = "info", duration = 3000) {
  let id: number | null = null;

  items.update((l) => {
    if (l.some((toast) => toast.message === message && toast.kind === kind)) {
      return l;
    }

    id = nextId++;
    return [
      ...l,
      { id, message, kind, progress: 100, paused: false },
    ];
  });

  if (id === null) return;
  playNotifySound();
  createDismissTimer(id, duration);
}

export const toasts = {
  subscribe: items.subscribe,
  info: (m: string, d?: number) => push(m, "info", d),
  success: (m: string, d?: number) => push(m, "success", d),
  error: (m: string, d?: number) => push(m, "error", d),
  warning: (m: string, d?: number) => push(m, "warning", d),
  dismiss,
  pause: (id: number) =>
    items.update((l) =>
      l.map((t) => (t.id === id ? { ...t, paused: true } : t)),
    ),
  resume: (id: number) =>
    items.update((l) =>
      l.map((t) => (t.id === id ? { ...t, paused: false } : t)),
    ),
};
