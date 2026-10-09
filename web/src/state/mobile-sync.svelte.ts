import { vynl } from "@lib/vynl";
import { t } from "@lib/i18n";
import { toasts } from "@lib/toast";
import type { MobileSyncStatus } from "@lib/types";

let _status = $state<MobileSyncStatus | null>(null);
let _loading = $state(false);
let _busy = $state(false);
let _rotating = $state(false);
let _uiOpen = $state(false);
let _loaded = false;

const SPIN_MIN_MS = 450;

function wait(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

export function getMobileSyncStatus(): MobileSyncStatus | null {
  return _status;
}

export function getMobileSyncLoading(): boolean {
  return _loading;
}

export function getMobileSyncBusy(): boolean {
  return _busy;
}

export function getMobileSyncRotating(): boolean {
  return _rotating;
}

export function getMobileSyncUiOpen(): boolean {
  return _uiOpen;
}

function errText(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

export async function refreshMobileSync(force = false): Promise<void> {
  if (_loading) return;
  if (_loaded && !force) return;
  _loading = true;
  try {
    _status = await vynl.getMobileSyncStatus();
    _loaded = true;
  } catch (e) {
    console.warn("[mobile-sync] failed to read status:", e);
  } finally {
    _loading = false;
  }
}

export async function setMobileSyncEnabled(enabled: boolean): Promise<void> {
  if (_busy) return;
  _busy = true;
  try {
    _status = await vynl.setMobileSyncEnabled(enabled);
    toasts.success(
      t(enabled ? "settings.mobileSyncTurnedOn" : "settings.mobileSyncTurnedOff"),
    );
  } catch (e) {
    toasts.error(t("settings.mobileSyncToggleFailed", { error: errText(e) }));
  } finally {
    _busy = false;
  }
}

export async function rotateMobileSyncPin(): Promise<void> {
  if (_rotating || _busy) return;
  _rotating = true;
  _busy = true;
  const started = Date.now();
  try {
    _status = await vynl.rotateMobileSyncPin();
    toasts.success(t("settings.mobileSyncPinRotated"));
  } catch (e) {
    toasts.error(t("settings.mobileSyncPinFailed", { error: errText(e) }));
  } finally {
    await wait(Math.max(0, SPIN_MIN_MS - (Date.now() - started)));
    _rotating = false;
    _busy = false;
  }
}

export function openMobileSyncUi(): void {
  _uiOpen = true;
  void refreshMobileSync();
}

export function closeMobileSyncUi(): void {
  _uiOpen = false;
}
