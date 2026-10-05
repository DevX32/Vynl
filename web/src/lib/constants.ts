import type { AudioFormat } from "./types";

export const BITRATES: Record<AudioFormat, number[] | null> = {
  mp3: [128, 192, 256, 320],
};

export const RPC_THROTTLE = 4000;

export interface EqPreset {
  id: string;
  labelKey: string;
  gains: number[];
}

export const EQ_PRESETS: readonly EqPreset[] = [
  { id: "flat", labelKey: "eq.presets.flat", gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
  { id: "bass", labelKey: "eq.presets.bass", gains: [8, 6, 4, 2, 0, 0, 0, 0, 1, 2] },
  { id: "treble", labelKey: "eq.presets.treble", gains: [-2, -1, 0, 0, 0, 1, 2, 4, 6, 7] },
  { id: "vocal", labelKey: "eq.presets.vocal", gains: [-3, -3, -1, 2, 4, 4, 3, 1, 0, -1] },
  { id: "rock", labelKey: "eq.presets.rock", gains: [5, 4, 3, 1, -1, -1, 1, 3, 4, 4] },
  { id: "podcast", labelKey: "eq.presets.podcast", gains: [-5, -4, -1, 3, 5, 5, 3, 0, -2, -4] },
  { id: "acoustic", labelKey: "eq.presets.acoustic", gains: [4, 3, 2, 0, 1, 2, 2, 3, 2, 1] },
  {
    id: "electronic",
    labelKey: "eq.presets.electronic",
    gains: [6, 5, 2, 0, -1, 1, 2, 3, 4, 5],
  },
  {
    id: "classical",
    labelKey: "eq.presets.classical",
    gains: [3, 2, 1, 0, -1, -1, 0, 2, 3, 4],
  },
  { id: "hiphop", labelKey: "eq.presets.hiphop", gains: [7, 6, 3, 2, -1, 1, 0, 2, 3, 3] },
  { id: "jazz", labelKey: "eq.presets.jazz", gains: [4, 3, 1, 2, -1, -1, 1, 2, 3, 3] },
  { id: "latin", labelKey: "eq.presets.latin", gains: [5, 4, 1, -1, -1, 1, 2, 3, 4, 3] },
  {
    id: "loudness",
    labelKey: "eq.presets.loudness",
    gains: [8, 6, 3, 0, -2, -2, 0, 3, 5, 6],
  },
  {
    id: "spoken",
    labelKey: "eq.presets.spoken",
    gains: [-6, -5, -2, 2, 4, 5, 4, 1, 0, -3],
  },
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
