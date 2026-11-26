export function evictOldest<K, V>(map: Map<K, V>, cap: number): void {
  while (map.size > cap) {
    const oldest = map.keys().next().value;
    if (oldest === undefined) break;
    map.delete(oldest);
  }
}
