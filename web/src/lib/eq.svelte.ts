import type { EqDesign } from "@lib/types";
import { vynl } from "@lib/vynl";

let _design = $state<EqDesign | null>(null);

export async function loadEqDesign(): Promise<boolean> {
  if (_design) return true;
  try {
    _design = await vynl.getEqDesign();
    return true;
  } catch (e) {
    console.warn("[EQ] could not load design from backend:", e);
    return false;
  }
}

export function eqBands(): readonly number[] {
  return _design?.freqs ?? [];
}

export function eqMinDb(): number {
  return _design?.minDb ?? 0;
}

export function eqMaxDb(): number {
  return _design?.maxDb ?? 0;
}

export function eqFlat(): number[] {
  return (_design?.freqs ?? []).map(() => 0);
}

const DISPLAY_SAMPLE_RATE = 48000;

interface Coeffs {
  b0: number;
  b1: number;
  b2: number;
  a1: number;
  a2: number;
}

const FLAT: Coeffs = { b0: 1, b1: 0, b2: 0, a1: 0, a2: 0 };

function peaking(w0: number, q: number, gainDb: number): Coeffs {
  const a = Math.pow(10, gainDb / 40);
  const cos = Math.cos(w0);
  const alpha = Math.sin(w0) / (2 * q);
  const a0 = 1 + alpha / a;
  return {
    b0: (1 + alpha * a) / a0,
    b1: (-2 * cos) / a0,
    b2: (1 - alpha * a) / a0,
    a1: (-2 * cos) / a0,
    a2: (1 - alpha / a) / a0,
  };
}

function shelf(
  w0: number,
  shape: number,
  gainDb: number,
  high: boolean,
): Coeffs {
  const amp = Math.pow(10, gainDb / 20);
  const m = shape;
  const k = Math.tan(w0 / 2);
  const k2 = k * k;
  const mid = m * Math.sqrt(amp) * k;
  const [b0, b1, b2] = high
    ? [amp + mid + k2, 2 * (k2 - amp), amp - mid + k2]
    : [1 + mid + amp * k2, 2 * (amp * k2 - 1), 1 - mid + amp * k2];
  const a0 = 1 + m * k + k2;
  const a1 = 2 * (k2 - 1);
  const a2 = 1 - m * k + k2;
  return { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0 };
}

function design(band: number, gainDb: number, sampleRate: number): Coeffs {
  const d = _design;
  if (!d) return FLAT;
  const freq = d.freqs[band];
  if (freq == null) return FLAT;
  if (
    Math.abs(gainDb) < d.flatDb ||
    sampleRate <= 0 ||
    freq >= sampleRate * 0.49
  ) {
    return FLAT;
  }
  const w0 = Math.min((2 * Math.PI * freq) / sampleRate, Math.PI * 0.999);
  const coeffs =
    band === d.lowShelf || band === d.highShelf
      ? shelf(w0, d.shape, gainDb, band === d.highShelf)
      : peaking(w0, d.qs[band], gainDb);
  const vals = [coeffs.b0, coeffs.b1, coeffs.b2, coeffs.a1, coeffs.a2];
  if (vals.some((v) => !Number.isFinite(v))) return FLAT;
  if (Math.abs(coeffs.a1) > 2 || Math.abs(coeffs.a2) >= 1) return FLAT;
  return coeffs;
}

export function eqResponseDb(bands: readonly number[], freq: number): number {
  const sr = DISPLAY_SAMPLE_RATE;
  const w = (2 * Math.PI * freq) / sr;
  const zr = Math.cos(w);
  const zi = -Math.sin(w);
  const z2r = 2 * zr * zr - 1;
  const z2i = 2 * zi * zr;

  let db = 0;
  const d = _design;
  if (!d) return 0;
  for (let i = 0; i < d.freqs.length; i++) {
    const c = design(i, bands[i] ?? 0, sr);
    const numMag = Math.hypot(
      c.b0 + c.b1 * zr + c.b2 * z2r,
      c.b1 * zi + c.b2 * z2i,
    );
    const denMag = Math.hypot(
      1 + c.a1 * zr + c.a2 * z2r,
      c.a1 * zi + c.a2 * z2i,
    );
    if (!(numMag > 0) || !(denMag > 0)) return 0;
    db += 20 * Math.log10(numMag / denMag);
  }
  return db;
}

function eqPreampDb(bands: readonly number[]): number {
  const trim = _design?.preampTrim ?? 0;
  let peak = 0;
  for (const g of bands) if (g > 0) peak = Math.max(peak, g);
  return peak <= 0 ? 0 : -(peak * trim);
}

function eqMaxBoostDb(bands: readonly number[]): number {
  let max = -Infinity;
  for (let f = 20; f <= 20000; f *= 1.04) {
    max = Math.max(max, eqResponseDb(bands, f));
  }
  return Number.isFinite(max) ? max : 0;
}

export function eqResidualBoostDb(bands: readonly number[]): number {
  return eqMaxBoostDb(bands) + eqPreampDb(bands);
}