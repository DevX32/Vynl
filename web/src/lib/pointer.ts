export function pointerRatio(el: HTMLElement, clientX: number): number {
  const rect = el.getBoundingClientRect();
  return clamp01((clientX - rect.left) / rect.width);
}

export function clamp01(value: number): number {
  return Math.min(1, Math.max(0, value));
}
