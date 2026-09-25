export function moveItem<T>(items: readonly T[], from: number, to: number): T[] {
  if (from === to) return [...items];
  if (from < 0 || to < 0 || from >= items.length || to >= items.length) {
    return [...items];
  }
  const next = [...items];
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
}

export function nearestIndex(centers: readonly number[], y: number): number | null {
  let closest: number | null = null;
  let closestDist = Infinity;
  for (let i = 0; i < centers.length; i += 1) {
    const dist = Math.abs(y - centers[i]);
    if (dist < closestDist) {
      closestDist = dist;
      closest = i;
    }
  }
  return closest;
}
