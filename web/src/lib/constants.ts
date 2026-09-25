import type { AudioFormat } from "./types";

export const BITRATES: Record<AudioFormat, number[] | null> = {
  mp3: [128, 192, 256, 320],
  m4a: [128, 192, 256, 320],
  opus: [128, 160, 192, 256, 320],
  flac: null,
  wav: null,
};

export const FORMAT_LABELS: Record<AudioFormat, string> = {
  mp3: "MP3",
  m4a: "M4A (AAC)",
  opus: "Opus",
  flac: "FLAC",
  wav: "WAV",
};

export const FORMAT_ORDER: AudioFormat[] = [
  "mp3",
  "m4a",
  "opus",
  "flac",
  "wav",
];

export const RPC_THROTTLE = 4000;
export const EQ_BANDS = [
  31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000,
] as const;

export const EQ_MIN_DB = -12;
export const EQ_MAX_DB = 12;

export const EQ_FLAT: number[] = EQ_BANDS.map(() => 0);

export interface EqPreset {
  id: string;
  label: string;
  gains: number[];
}

export const EQ_PRESETS: readonly EqPreset[] = [
  { id: "flat", label: "Flat", gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
  { id: "bass", label: "Bass Boost", gains: [8, 7, 5, 3, 1, 0, 0, 0, 0, 0] },
  { id: "treble", label: "Treble Boost", gains: [0, 0, 0, 0, 0, 1, 2, 4, 6, 8] },
  { id: "vocal", label: "Vocal", gains: [-3, -3, -1, 2, 4, 4, 3, 1, 0, -1] },
  { id: "rock", label: "Rock", gains: [5, 4, 3, 1, -1, -1, 1, 3, 4, 5] },
  { id: "podcast", label: "Podcast", gains: [-5, -4, -1, 3, 5, 5, 3, 0, -2, -4] },
];

export const DURATION_TOLERANCE = 0.05;

export const QUOTES = [
  "where words fail, music speaks",
  "music is the shorthand of emotion",
  "life is a song. love is the music",
  "without music, life would be a mistake",
  "music is the universal language of mankind",
  "one good thing about music, when it hits you, you feel no pain",
  "music expresses that which cannot be put into words",
] as const;
