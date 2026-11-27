import { attachConsole } from "@tauri-apps/plugin-log";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { vynl } from "./vynl";
import { restoreNowPlaying, setPlayHandler } from "@state/now-playing.svelte";
import { initSettings } from "@state/settings.svelte";
import { flushCloudSyncPush, refreshSyncStatus, retryPendingSync } from "@state/sync.svelte";
import { hydrateTaste } from "@state/taste.svelte";
import { t } from "./i18n";
import { refreshLibrary } from "@state/library.svelte";
import { playTrack } from "./playback.svelte";
import { handleGlobalKey } from "@state/shortcuts.svelte";
import { wasOnboarded } from "./onboarding.svelte";
import { initAppUpdate } from "@state/app-update.svelte";
import { toasts } from "./toast";
import { refreshPlaylists } from "@state/playlists.svelte";

const SPLASH_MIN_MS = 2500;
const splashReady = Date.now();

export function hideSplash(minMs: number = SPLASH_MIN_MS): void {
  const elapsed = Date.now() - splashReady;
  const wait = Math.max(0, minMs - elapsed);
  setTimeout(() => {
    const splash = document.getElementById("splash");
    if (splash) {
      splash.classList.add("hide");
      setTimeout(() => splash.remove(), 700);
    }
  }, wait);
}

const handledConfirmTokens = new Set<string>();

export async function handleConfirmUrl(url: string): Promise<void> {
  if (!url.startsWith("vynl://confirm")) return;
  const raw = url.replace(/^vynl:\/\/confirm[?&]/, "");
  const params = new URLSearchParams(raw);
  const token = params.get("token_hash") ?? params.get("token");
  if (!token || handledConfirmTokens.has(token)) return;
  handledConfirmTokens.add(token);
  const type = params.get("type") ?? "signup";
  try {
    const ok = await vynl.syncConfirmEmail(token, type);
    if (ok) toasts.success(t("account.emailConfirmed"));
    else toasts.error(t("account.emailConfirmFailed"));
  } catch (e) {
    handledConfirmTokens.delete(token);
    toasts.error(t("account.emailConfirmFailed"));
    console.warn("email confirmation failed", e);
  }
}

async function bootApp(setOnboarded: (v: boolean) => void): Promise<(() => void) | undefined> {
  setPlayHandler((id, queue) => playTrack(id, queue));
  setOnboarded(wasOnboarded());
  try {
    await initSettings();
    void vynl.rpcUpdate(null).catch(() => {});
    await Promise.all([
      refreshPlaylists().catch((e) => console.warn("playlist refresh failed:", e)),
      refreshLibrary().catch((e) => console.warn("library refresh failed:", e)),
      restoreNowPlaying().catch((e) => console.warn("restore failed:", e)),
    ]);
  } catch (e) {
    console.warn("boot failed", e);
  } finally {
    hideSplash();
  }
  const off = vynl.onDownloadEvent((e) => {
    if (e.type === "finished") {
      void refreshPlaylists();
      void refreshLibrary();
    }
  });
  void initAppUpdate();
  void (async () => {
    try {
      const session = await refreshSyncStatus();
      if (!session.signedIn) return;
      await hydrateTaste();
    } catch (e) {
      console.warn("sync status failed", e);
    }
  })();
  return () => { off(); };
}

export function setupApp(opts: {
  isMini: boolean;
  setOnboarded: (v: boolean) => void;
  onToggleFullscreen: () => void;
}): () => void {
  if (opts.isMini) {
    hideSplash(200);
    return () => {};
  }

  void getCurrentWindow().show().catch(() => {});

  let offConsole: (() => void) | undefined;
  let consoleDisposed = false;
  if (import.meta.env.PROD) {
    void attachConsole().then(
      (off) => { if (consoleDisposed) off(); else offConsole = off; },
      () => {},
    );
  }

  let offDeepLink: (() => void) | undefined;
  let deepLinkDisposed = false;
  void (async () => {
    try {
      const url = await vynl.takePendingConfirmUrl();
      if (url) await handleConfirmUrl(url);
      const urls = await getCurrent();
      for (const startUrl of urls ?? []) await handleConfirmUrl(startUrl);
    } catch (e) {
      console.warn("pending confirm failed", e);
    }
    try {
      const unlisten = await onOpenUrl((urls) => {
        for (const url of urls) void handleConfirmUrl(url);
      });
      if (deepLinkDisposed) unlisten();
      else offDeepLink = unlisten;
    } catch (e) {
      console.warn("deep link listener unavailable", e);
    }
  })();

  const offToast = vynl.onToast((summary) => {
    if (summary.cancelled) {
      toasts.info(t("vault.downloadCancelled"));
    } else if (summary.failed > 0) {
      toasts.warning(t("vault.downloadPartial", { done: summary.done, failed: summary.failed }));
    } else {
      toasts.success(t("vault.downloadComplete", {
        n: summary.done,
        unit: summary.done === 1 ? t("common.trackLc") : t("common.tracks"),
      }));
    }
  });
  window.addEventListener("keydown", handleGlobalKey);
  window.addEventListener("pagehide", flushCloudSyncPush);
  window.addEventListener("online", retryPendingSync);
  document.addEventListener("vynl:toggle-fullscreen", opts.onToggleFullscreen);
  let cleanup: (() => void) | undefined;
  let disposeMini: (() => void) | undefined;
  let disposed = false;
  void import("@state/mini-bridge.svelte").then((m) => {
    if (disposed) m.initMiniBridge()();
    else disposeMini = m.initMiniBridge();
  });
  void bootApp(opts.setOnboarded)
    .then((c) => { if (disposed) c?.(); else cleanup = c; })
    .catch(console.error);
  return () => {
    disposed = true;
    consoleDisposed = true;
    offConsole?.();
    deepLinkDisposed = true;
    offDeepLink?.();
    disposeMini?.();
    cleanup?.();
    offToast();
    setPlayHandler(null);
    window.removeEventListener("keydown", handleGlobalKey);
    window.removeEventListener("pagehide", flushCloudSyncPush);
    window.removeEventListener("online", retryPendingSync);
    document.removeEventListener("vynl:toggle-fullscreen", opts.onToggleFullscreen);
  };
}
