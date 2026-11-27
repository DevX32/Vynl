export const IS_MINI =
  typeof window !== "undefined" && window.location.hash.startsWith("#mini");

export const ps = {
  playGeneration: Date.now(),
  lastPositionCheckpoint: Date.now(),
  reconcilePending: false,
  pauseIntent: 0,
  resumeToken: 0,
  resumePending: 0,
  lastPrebufferCheck: 0,
  playerUnlisten: null as (() => void) | null,
};

export const PREBUFFER_CHECK_MS = 5_000;
