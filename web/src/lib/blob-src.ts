import { convertFileSrc } from "@tauri-apps/api/core";
import { evictOldest } from "@lib/cache";
import { vynl } from "@lib/vynl";

const MAX_CACHE_ENTRIES = 500;
const remote = new Map<string, Promise<string>>();
const resolved = new Map<string, string>();

export function clearResolvedCovers(): void {
  resolved.clear();
  remote.clear();
}

function evictIfNeeded(map: Map<string, unknown>): void {
  evictOldest(map, MAX_CACHE_ENTRIES);
}

function upsizeCover(url: string): string {
  const trimmed = url.trim();
  const start = trimmed.indexOf("=w");
  if (start < 0) return trimmed;

  let i = start + 2;
  while (i < trimmed.length && trimmed[i] >= "0" && trimmed[i] <= "9") i++;
  const width = Number.parseInt(trimmed.slice(start + 2, i), 10);
  if (!Number.isFinite(width) || !trimmed.slice(i).startsWith("-h"))
    return trimmed;

  let j = i + 2;
  const hStart = j;
  while (j < trimmed.length && trimmed[j] >= "0" && trimmed[j] <= "9") j++;
  if (j === hStart) return trimmed;
  const height = Number.parseInt(trimmed.slice(hStart, j), 10);
  if (!Number.isFinite(height)) return trimmed;

  const seen = new Set<string>();
  const params: string[] = [];
  for (const part of trimmed.slice(j).split("-")) {
    if (part === "" || seen.has(part)) continue;
    seen.add(part);
    params.push(part);
  }

  const nw = Math.max(width, 512);
  const nh = Math.max(height, 512);
  const dashCount = (trimmed.slice(j).match(/-/g) ?? []).length;
  if (nw === width && nh === height && params.length === dashCount)
    return trimmed;

  return `${trimmed.slice(0, start)}=w${nw}-h${nh}${params.map((p) => `-${p}`).join("")}`;
}

function cacheRemote(url: string): Promise<string> {
  let pending = remote.get(url);
  if (!pending) {
    pending = vynl
      .cacheCover(url)
      .then((local) => {
        remote.delete(url);
        return local;
      })
      .catch((error) => {
        remote.delete(url);
        throw error;
      });
    remote.set(url, pending);
    evictIfNeeded(remote);
  }
  return pending;
}

export function blobSrc(
  node: HTMLImageElement,
  path: string | null | undefined,
): {
  update: (newPath: string | null | undefined) => void;
  destroy: () => void;
} {
  let token = 0;
  let current: string | null = null;

  function setSrc(src: string, mine: number): void {
    if (mine !== token || current === src) return;
    current = src;
    node.src = src;
  }

  function load(p: string | null | undefined): void {
    const mine = ++token;
    if (!p) {
      current = null;
      node.removeAttribute("src");
      return;
    }

    if (!/^https?:\/\//i.test(p)) {
      setSrc(convertFileSrc(p), mine);
      return;
    }

    const url = upsizeCover(p);
    const known = resolved.get(url);
    if (known) {
      setSrc(convertFileSrc(known), mine);
      return;
    }

    node.referrerPolicy = "no-referrer";
    let remoteLoaded = false;
    void cacheRemote(url).then(
      (local) => {
        resolved.set(url, local);
        evictIfNeeded(resolved);
        if (!remoteLoaded) setSrc(convertFileSrc(local), mine);
      },
      () => {},
    );

    const preload = new Image();
    preload.referrerPolicy = "no-referrer";
    preload.onload = () => {
      remoteLoaded = true;
      if (!resolved.has(url)) setSrc(url, mine);
    };
    preload.onerror = () => {};
    preload.src = url;
  }

  load(path);

  return {
    update(newPath: string | null | undefined) {
      load(newPath);
    },
    destroy() {
      token++;
      current = null;
    },
  };
}
