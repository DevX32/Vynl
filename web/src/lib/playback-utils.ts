export function normalizeDuration(value: number | undefined): number {
  return typeof value === "number" && Number.isFinite(value) && value > 0
    ? value
    : 0;
}

export function isPlayInterrupt(err: unknown): boolean {
  return (
    err instanceof DOMException &&
    (err.name === "AbortError" || err.name === "NotAllowedError")
  );
}

export function isSeekUnsupported(error: unknown): boolean {
  const message = String(error).toLowerCase();
  return message.includes("seek:") && message.includes("support");
}

export function extractVideoId(value: string): string | null {
  const raw = value.trim();
  if (!raw) return null;
  if (/^[a-zA-Z0-9_-]{11}$/.test(raw)) return raw;
  try {
    const url = new URL(raw);
    const host = url.hostname.replace(/^www\./, "").toLowerCase();
    if (host === "youtu.be") {
      const id = url.pathname.split("/").filter(Boolean)[0];
      return id && /^[a-zA-Z0-9_-]{11}$/.test(id) ? id : null;
    }
    if (host === "youtube.com" || host.endsWith(".youtube.com")) {
      const v = url.searchParams.get("v");
      if (v && /^[a-zA-Z0-9_-]{11}$/.test(v)) return v;
      const parts = url.pathname.split("/").filter(Boolean);
      const idx = parts.findIndex((p) => p === "shorts" || p === "embed");
      const id = idx >= 0 ? parts[idx + 1] : null;
      return id && /^[a-zA-Z0-9_-]{11}$/.test(id) ? id : null;
    }
  } catch {}
  return null;
}
