const URL_PATTERN = /https?:\/\/\S+/gi;
const YT_API_PATTERN = /\b(?:music\.youtube\.com|www\.youtube\.com|youtube\.com)\b[^\s]*/gi;

const CAUSES: { test: RegExp; reason: string }[] = [
  {
    test: /dns error|no such host|failed to lookup address|name or service not known/i,
    reason: "no internet connection",
  },
  {
    test: /connection refused|connection reset|connection closed|network is unreachable/i,
    reason: "the connection was interrupted",
  },
  {
    test: /timed out|timeout/i,
    reason: "the request timed out",
  },
  {
    test: /certificate|ssl|tls/i,
    reason: "the secure connection failed",
  },
  {
    test: /403|forbidden|unauthorized|401/i,
    reason: "YouTube rejected the request",
  },
  {
    test: /429|too many requests|rate.?limit/i,
    reason: "too many requests, try again shortly",
  },
  {
    test: /quota|exhausted/i,
    reason: "the YouTube API quota is used up",
  },
];

function stripUrls(message: string): string {
  return message
    .replace(URL_PATTERN, "")
    .replace(YT_API_PATTERN, "")
    .replace(/\s+/g, " ")
    .trim();
}

export function friendlyError(error: unknown, fallback = ""): string {
  if (error == null) return fallback;
  const raw = typeof error === "string" ? error : String(error);
  const cleaned = stripUrls(raw);
  if (!cleaned) return fallback;

  const tail = cleaned
    .split(/\s*->\s*|\s*:\s*(?=[a-z]+ (?:error|failed|refused))/i)
    .map((part) => part.trim())
    .filter(Boolean)
    .pop();
  const candidate = tail && tail.length >= 8 ? tail : cleaned;

  for (const cause of CAUSES) {
    if (cause.test.test(candidate)) return cause.reason;
  }

  const withoutPrefix = candidate.replace(
    /^(song|album|artist|playlist)( (?:search|lookup|page))? failed:\s*/i,
    "",
  );

  const trimmed = withoutPrefix.trim();
  if (!trimmed) return fallback;
  if (trimmed.length > 160) return `${trimmed.slice(0, 157).trimEnd()}...`;
  return trimmed;
}
