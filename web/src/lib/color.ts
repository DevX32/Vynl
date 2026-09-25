export const DEFAULT_ACCENT_COLOR = "#a894e8";

export const ACCENT_PRESETS = [
  "#a894e8", // default (lavender)
  "#7aa2f7", // blue
  "#8fd694", // green
  "#e8c86a", // gold
  "#e8945c", // orange
  "#ff6b61", // red
  "#f06ba8", // pink
] as const;

const HEX_RE = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i;

export function isValidHexColor(value: string): boolean {
  return HEX_RE.test(value.trim());
}

function hexToRgb(hex: string): [number, number, number] | null {
  const clean = hex.trim().replace("#", "");
  const full =
    clean.length === 3
      ? clean.split("").map((c) => c + c).join("")
      : clean;
  if (full.length !== 6) return null;
  const num = parseInt(full, 16);
  if (Number.isNaN(num)) return null;
  return [(num >> 16) & 255, (num >> 8) & 255, num & 255];
}

function componentToHex(c: number): string {
  return Math.round(clamp(c, 0, 255)).toString(16).padStart(2, "0");
}

function clamp(n: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, n));
}

export function normalizeHex(hex: string): string {
  const rgb = hexToRgb(hex);
  if (!rgb) return DEFAULT_ACCENT_COLOR;
  return `#${componentToHex(rgb[0])}${componentToHex(rgb[1])}${componentToHex(rgb[2])}`;
}

interface Hsv {
  h: number;
  s: number;
  v: number;
}

export function hexToHsv(hex: string): Hsv | null {
  const rgb = hexToRgb(hex);
  if (!rgb) return null;
  const [r, g, b] = rgb.map((c) => c / 255);
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const d = max - min;
  let h = 0;
  if (d !== 0) {
    if (max === r) h = ((g - b) / d) % 6;
    else if (max === g) h = (b - r) / d + 2;
    else h = (r - g) / d + 4;
    h *= 60;
    if (h < 0) h += 360;
  }
  const s = max === 0 ? 0 : d / max;
  const v = max;
  return { h, s: s * 100, v: v * 100 };
}

export function hsvToHex(h: number, s: number, v: number): string {
  const sat = clamp(s, 0, 100) / 100;
  const val = clamp(v, 0, 100) / 100;
  const hue = ((h % 360) + 360) % 360;
  const c = val * sat;
  const x = c * (1 - Math.abs(((hue / 60) % 2) - 1));
  const m = val - c;
  let [r, g, b] = [0, 0, 0];
  if (hue < 60) [r, g, b] = [c, x, 0];
  else if (hue < 120) [r, g, b] = [x, c, 0];
  else if (hue < 180) [r, g, b] = [0, c, x];
  else if (hue < 240) [r, g, b] = [0, x, c];
  else if (hue < 300) [r, g, b] = [x, 0, c];
  else [r, g, b] = [c, 0, x];
  return `#${componentToHex((r + m) * 255)}${componentToHex((g + m) * 255)}${componentToHex((b + m) * 255)}`;
}

function relativeLuminance([r, g, b]: [number, number, number]): number {
  const chan = (c: number): number => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * chan(r) + 0.7152 * chan(g) + 0.0722 * chan(b);
}

type Rgb = [number, number, number];

const ACCENT_TEXT_DARK: Rgb = [22, 17, 40]; 
const ACCENT_TEXT_LIGHT: Rgb = [245, 243, 255]; 

const ANIM_DURATION = 450; 

function rgbToHexString([r, g, b]: Rgb): string {
  return `#${componentToHex(r)}${componentToHex(g)}${componentToHex(b)}`;
}

function accentTextTarget(luminosity: number): Rgb {
  return luminosity > 0.55 ? ACCENT_TEXT_DARK : ACCENT_TEXT_LIGHT;
}

function parseCssColor(str: string): Rgb | null {
  const s = str.trim();
  const hex = hexToRgb(s);
  if (hex) return hex;
  const rgba = s.match(/^rgba?\(([^)]+)\)$/i);
  if (rgba) {
    const parts = rgba[1].split(",").map((x) => parseFloat(x));
    if (parts.length >= 3 && parts.slice(0, 3).every((n) => !Number.isNaN(n))) {
      return [parts[0], parts[1], parts[2]];
    }
  }
  return null;
}

function currentAccentBase(): Rgb {
  const raw = getComputedStyle(document.documentElement).getPropertyValue(
    "--accent",
  );
  return parseCssColor(raw) ?? (hexToRgb(DEFAULT_ACCENT_COLOR) as Rgb);
}

function currentAccentText(): Rgb {
  const raw = getComputedStyle(document.documentElement).getPropertyValue(
    "--accent-text",
  );
  return parseCssColor(raw) ?? ACCENT_TEXT_DARK;
}

function writeAccent(rgb: Rgb, text: Rgb): void {
  const [r, g, b] = rgb;
  const rgbStr = `${Math.round(r)}, ${Math.round(g)}, ${Math.round(b)}`;
  const root = document.documentElement.style;
  root.setProperty("--accent", rgbToHexString(rgb));
  root.setProperty("--accent-soft", `rgba(${rgbStr}, 0.11)`);
  root.setProperty("--accent-glow", `rgba(${rgbStr}, 0.38)`);
  root.setProperty("--accent-text", rgbToHexString(text));
}

function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

function lerpRgb(from: Rgb, to: Rgb, t: number): Rgb {
  return [lerp(from[0], to[0], t), lerp(from[1], to[1], t), lerp(from[2], to[2], t)];
}

function sameRgb(a: Rgb, b: Rgb): boolean {
  return (
    Math.round(a[0]) === Math.round(b[0]) &&
    Math.round(a[1]) === Math.round(b[1]) &&
    Math.round(a[2]) === Math.round(b[2])
  );
}

function easeOutCubic(t: number): number {
  return 1 - Math.pow(1 - t, 3);
}

function prefersReducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

let _accentRaf = 0;

export function applyAccentColor(hex: string, animate = true): void {
  const color = isValidHexColor(hex) ? hex.trim() : DEFAULT_ACCENT_COLOR;
  const target = hexToRgb(color);
  if (!target) return;

  if (_accentRaf) cancelAnimationFrame(_accentRaf);

  const targetText = accentTextTarget(relativeLuminance(target));

  if (!animate || prefersReducedMotion() || sameRgb(currentAccentBase(), target)) {
    writeAccent(target, targetText);
    return;
  }

  const start = currentAccentBase();
  const startText = currentAccentText();
  const startNow = performance.now();

  const step = (now: number): void => {
    const t = Math.min(1, (now - startNow) / ANIM_DURATION);
    const e = easeOutCubic(t);
    writeAccent(lerpRgb(start, target, e), lerpRgb(startText, targetText, e));
    if (t < 1) {
      _accentRaf = requestAnimationFrame(step);
    } else {
      _accentRaf = 0;
    }
  };
  _accentRaf = requestAnimationFrame(step);
}
