export function pointerRatio(el: HTMLElement, clientX: number): number {
  const rect = el.getBoundingClientRect();
  if (rect.width <= 0) return 0;
  return clamp01((clientX - rect.left) / rect.width);
}

export function clamp01(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.min(1, Math.max(0, value));
}
