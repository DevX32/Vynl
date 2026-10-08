import { vynl } from "@lib/vynl";
import { t } from "@lib/i18n";
import { toasts } from "@lib/toast";
import type { UpdateStatus } from "@lib/types";

let _status = $state<UpdateStatus | null>(null);
let _checking = $state(false);
let _installing = $state(false);
let _currentVersion = $state<string | null>(null);
let _unlisten: (() => void) | null = null;
let _checkedOnce = false;
let _checkedAt: number | null = null;

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

function ensureListener(): void {
  if (_unlisten) return;
  _unlisten = vynl.onUpdateStatus((next) => {
    applyStatus(next);
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
    } else if (notify) {
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
  if (_installing) return;
  _installing = true;
  try {
    await vynl.installAppUpdate();
  } catch (e) {
    toasts.error(t("update.checkFailed", { error: errText(e) }));
    if (_status) {
      applyStatus({
        ..._status,
        downloading: false,
        ready: false,
        error: errText(e),
      });
    }
  } finally {
    _installing = false;
  }
}

export async function restartAndInstall(): Promise<void> {
  try {
    await vynl.restartApp();
  } catch (e) {
    toasts.error(t("update.checkFailed", { error: errText(e) }));
  }
}