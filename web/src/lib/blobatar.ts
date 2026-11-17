import { blobatarUri } from "blobatar/uri";
import type { BlobatarOptions } from "blobatar/blob";

export const AVATAR_STORAGE_KEY = "vynl.avatar.v1";

let revision = 0;

export function avatarRevision(): number {
  return revision;
}

export type AvatarBackground = "default" | "none" | "square" | "circle" | "squircle";

export interface AvatarCustomization {
  seed: string;
  shape: string | null;
  hue: number | null;
  tone: number | null;
  background: AvatarBackground;
}

export const SHAPE_BANDS: Record<string, number> = {
  round: 0.11,
  organic: 0.35,
  boxy: 0.54,
  capsule: 0.65,
  nub: 0.745,
  cloud: 0.825,
  droplet: 0.888,
  hexagon: 0.933,
  sun: 0.965,
  triangle: 0.99,
};

export const SHAPE_OPTIONS = Object.keys(SHAPE_BANDS);

const DEFAULT_CUSTOM: AvatarCustomization = {
  seed: "",
  shape: null,
  hue: null,
  tone: null,
  background: "default",
};

function read(): AvatarCustomization {
  if (typeof localStorage === "undefined") return { ...DEFAULT_CUSTOM };
  try {
    const raw = localStorage.getItem(AVATAR_STORAGE_KEY);
    if (!raw) return { ...DEFAULT_CUSTOM };
    const parsed = JSON.parse(raw) as Partial<AvatarCustomization>;
    return {
      seed: typeof parsed.seed === "string" ? parsed.seed : "",
      shape: typeof parsed.shape === "string" ? parsed.shape : null,
      hue:
        typeof parsed.hue === "number" && Number.isFinite(parsed.hue)
          ? parsed.hue
          : null,
      tone:
        typeof parsed.tone === "number" && Number.isFinite(parsed.tone)
          ? parsed.tone
          : null,
      background: parsed.background ?? "default",
    };
  } catch {
    return { ...DEFAULT_CUSTOM };
  }
}

export function getAvatarCustom(): AvatarCustomization {
  return read();
}

export function saveAvatarCustom(patch: Partial<AvatarCustomization>): void {
  if (typeof localStorage === "undefined") return;
  const next = { ...read(), ...patch };
  localStorage.setItem(AVATAR_STORAGE_KEY, JSON.stringify(next));
  revision += 1;
}

export function resetAvatarCustom(): void {
  if (typeof localStorage === "undefined") return;
  localStorage.removeItem(AVATAR_STORAGE_KEY);
  revision += 1;
}

export function avatarUri(
  seed: string,
  opts?: BlobatarOptions,
): string {
  const custom = read();
  const seedToUse = custom.seed.trim() || seed;
  const traits: Record<string, number> = {};
  if (custom.shape && SHAPE_BANDS[custom.shape] !== undefined) {
    traits.shape = SHAPE_BANDS[custom.shape];
  }
  const merged: BlobatarOptions = {
    size: opts?.size,
    ...opts,
  };
  if (custom.hue !== null) merged.hue = custom.hue;
  if (custom.tone !== null) merged.tone = custom.tone;
  if (custom.background === "none") merged.background = false;
  else if (custom.background !== "default") merged.background = custom.background;
  if (Object.keys(traits).length > 0) {
    merged.traits = { ...(opts?.traits as Record<string, number> | undefined), ...traits };
  }
  return blobatarUri(seedToUse, merged);
}

export function randomAvatarSeed(): string {
  const alphabet = "abcdefghijkmnopqrstuvwxyz23456789";
  let out = "";
  const buf = new Uint8Array(8);
  crypto.getRandomValues(buf);
  for (const byte of buf) {
    out += alphabet[byte % alphabet.length];
  }
  return out;
}
