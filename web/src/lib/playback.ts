export const SPEED_MIN = 0.5;
export const SPEED_MAX = 2;

export const SPEED_STEPS: readonly number[] = [0.5, 0.75, 1, 1.25, 1.5, 2];

export const SLEEP_STEPS: readonly number[] = [15, 30, 45, 60];

export function normalizeSpeed(value: number): number {
  if (!Number.isFinite(value)) return 1;
  const clamped = Math.min(SPEED_MAX, Math.max(SPEED_MIN, value));
  return Math.round(clamped * 100) / 100;
}

export function formatSpeed(value: number): string {
  const speed = normalizeSpeed(value);
  return `${Number.isInteger(speed) ? speed.toFixed(0) : speed.toFixed(2).replace(/0+$/, "").replace(/\.$/, "")}×`;
}

export function formatSleepRemaining(ms: number): string {
  const total = Math.max(0, Math.ceil(ms / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  }
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}
