import { convertFileSrc } from "@tauri-apps/api/core";

function artUrl(cover: string): string {
  return /^https?:\/\//i.test(cover) ? cover : convertFileSrc(cover);
}

const MAX_DIM = 96;

function componentToHex(c: number): string {
  return Math.round(Math.min(255, Math.max(0, c)))
    .toString(16)
    .padStart(2, "0");
}

function rgbToHex(r: number, g: number, b: number): string {
  return `#${componentToHex(r)}${componentToHex(g)}${componentToHex(b)}`;
}

export async function extractVibrantColor(
  cover: string | null | undefined,
): Promise<string | null> {
  if (!cover) return null;

  let img: HTMLImageElement;
  try {
    img = await loadImage(artUrl(cover));
  } catch {
    return null;
  }
  if (!img.naturalWidth || !img.naturalHeight) return null;

  const w = Math.min(img.naturalWidth, MAX_DIM);
  const h = Math.max(1, Math.round(w * (img.naturalHeight / img.naturalWidth)));

  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) return null;
  ctx.drawImage(img, 0, 0, w, h);

  let data: ImageData;
  try {
    data = ctx.getImageData(0, 0, w, h);
  } catch {
    return null;
  }

  const BINS = 24;
  const bins = Array.from({ length: BINS }, () => ({
    r: 0,
    g: 0,
    b: 0,
    n: 0,
    s: 0,
  }));

  const px = data.data;
  for (let i = 0; i < px.length; i += 4) {
    const r = px[i];
    const g = px[i + 1];
    const b = px[i + 2];

    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    const l = (max + min) / 510; 
    if (l < 0.22 || l > 0.9) continue;

    const d = max - min;
    const s = max === 0 ? 0 : d / max;
    if (s < 0.4) continue;

    let h = 0;
    if (d !== 0) {
      if (max === r) h = ((g - b) / d) % 6;
      else if (max === g) h = (b - r) / d + 2;
      else h = (r - g) / d + 4;
      h *= 60;
      if (h < 0) h += 360;
    }

    const bin = Math.min(BINS - 1, Math.floor((h / 360) * BINS));
    const c = bins[bin];
    c.r += r;
    c.g += g;
    c.b += b;
    c.n += 1;
    c.s += s;
  }

  let best: { r: number; g: number; b: number; n: number } | null = null;
  let bestScore = -1;
  for (const c of bins) {
    if (c.n < 4) continue;
    const avgS = c.s / c.n;
    const score = c.n * (0.5 + avgS);
    if (score > bestScore) {
      bestScore = score;
      best = c;
    }
  }
  if (!best) return null;

  const { r, g, b } = best;
  return rgbToHex(r / best.n, g / best.n, b / best.n);
}

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    if (/^https?:/i.test(src)) img.crossOrigin = "anonymous";
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error("failed to load art"));
    img.src = src;
  });
}