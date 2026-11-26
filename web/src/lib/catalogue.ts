export const CATALOGUE_PREFIX = "ytm:";

export function cataloguePath(id: string): string {
  return `${CATALOGUE_PREFIX}${id}`;
}

export function catalogueIdFromPath(
  path: string | null | undefined,
): string | null {
  if (!path || !path.startsWith(CATALOGUE_PREFIX)) return null;
  const id = path.slice(CATALOGUE_PREFIX.length);
  return id.length > 0 ? id : null;
}

export function isCataloguePath(path: string): boolean {
  return path.startsWith(CATALOGUE_PREFIX);
}
