import { vynl } from "@lib/vynl";
import { t } from "@lib/i18n";
import { toasts } from "@lib/toast";
import type { UpdateStatus } from "@lib/types";

let _status = $state<UpdateStatus | null>(null);
let _checking = $state(false);
let _installing = $state(false);
let _restarting = $state(false);
let _currentVersion = $state<string | null>(null);
let _unlisten: (() => void) | null = null;
let _checkedOnce = false;
let _checkedAt: number | null = null;
let _restartTimer: ReturnType<typeof setTimeout> | null = null;

const RESTART_DELAY_MS = 900;
const RESTART_FALLBACK_MS = 10_000;

export function getUpdateCheckedAt(): number | null {
  return _checkedAt;
}

export function getUpdateStatus(): UpdateStatus | null {
  return _status;
}

export function getUpdateChecking(): boolean {
  return _checking;
}

export function getUpdateInstalling(): boolean {
  return _installing;
}

export function getUpdateRestarting(): boolean {
  return _restarting;
}

export function getCurrentVersion(): string | null {
  return _currentVersion;
}

export function getUpdateAvailable(): boolean {
  return !!_status?.available && !_status?.downloading && !_status?.ready;
}

function applyStatus(next: UpdateStatus): void {
  _status = next;
  _currentVersion = next.currentVersion || null;
}

function errText(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

function disarmRestart(): void {
  if (_restartTimer !== null) {
    clearTimeout(_restartTimer);
    _restartTimer = null;
  }
  _restarting = false;
}

function armRestart(delay: number): void {
  if (_restartTimer !== null || _restarting) return;
  _installing = true;
  _restartTimer = setTimeout(() => {
    _restartTimer = null;
    _restarting = true;
    void vynl.restartApp().catch((e: unknown) => {
      disarmRestart();
      _installing = false;
      toasts.error(t("update.checkFailed", { error: errText(e) }));
    });
  }, delay);
}

function ensureListener(): void {
  if (_unlisten) return;
  _unlisten = vynl.onUpdateStatus((next) => {
    applyStatus(next);
    if (next.error) {
      disarmRestart();
      _installing = false;
    } else if (next.ready) {
      armRestart(RESTART_FALLBACK_MS);
    }
  });
}

export async function initAppUpdate(): Promise<void> {
  ensureListener();
  if (_checkedOnce) return;
  _checkedOnce = true;
  await checkForAppUpdate();
}

export async function checkForAppUpdate(force = true, notify = false): Promise<void> {
  if (_checking) return;
  _checking = true;
  try {
    const next = await vynl.checkAppUpdate(force);
    applyStatus(next);
    if (next.error) {
      toasts.error(t("update.checkFailed", { error: next.error }));
    } else if (notify && !next.available) {
      toasts.success(t("update.upToDate"));
    }
  } catch (e) {
    toasts.error(t("update.checkFailed", { error: errText(e) }));
  } finally {
    _checking = false;
    _checkedAt = Date.now();
  }
}

export async function installAppUpdate(): Promise<void> {
  if (_installing || _restarting) return;
  _installing = true;
  try {
    await vynl.installAppUpdate();
  } catch (e) {
    disarmRestart();
    _installing = false;
    toasts.error(t("update.checkFailed", { error: errText(e) }));
    if (_status) {
      applyStatus({
        ..._status,
        downloading: false,
        ready: false,
        error: errText(e),
      });
    }
    return;
  }
  disarmRestart();
  armRestart(RESTART_DELAY_MS);
}

export async function restartAndInstall(): Promise<void> {
  try {
    await vynl.restartApp();
  } catch (e) {
    toasts.error(t("update.checkFailed", { error: errText(e) }));
  }
}