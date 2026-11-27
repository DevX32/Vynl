import { vynl } from "@lib/vynl";
import type { SyncStatus } from "@lib/types";

let _status = $state<SyncStatus>({
  signedIn: false,
  email: null,
  userId: null,
  lastSyncedAt: null,
  lastError: null,
  pending: false,
});
let _loaded = $state(false);
let _syncing = $state(false);

export function getSyncStatus(): SyncStatus {
  return _status;
}

export function isSyncStatusLoaded(): boolean {
  return _loaded;
}

export function isCloudSyncing(): boolean {
  return _syncing;
}

export function setSyncStatus(next: SyncStatus): void {
  _status = next;
  _loaded = true;
}

export async function refreshSyncStatus(): Promise<SyncStatus> {
  const next = await vynl.syncStatus();
  setSyncStatus(next);
  return next;
}

let _pushTimer: ReturnType<typeof setTimeout> | null = null;
let _pushing = false;
let _pushQueued = false;
let _syncingNow = false;

function updateSyncing(): void {
  _syncing = _pushing || _syncingNow;
}

async function runCloudSyncPush(): Promise<void> {
  if (!_status.signedIn) return;
  if (_pushing) {
    _pushQueued = true;
    return;
  }
  _pushing = true;
  updateSyncing();
  try {
    await vynl.syncPush();
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    if (msg.includes("Conflict:")) {
      _status = { ..._status, pending: true, lastError: msg };
      void vynl.syncPull().then(() => refreshSyncStatus()).catch(() => {});
    } else {
      console.warn("[sync] cloud push failed", e);
    }
  } finally {
    await refreshSyncStatus().catch(() => {});
    _pushing = false;
    updateSyncing();
    if (_pushQueued) {
      _pushQueued = false;
      void runCloudSyncPush();
    }
  }
}

export function scheduleCloudSyncPush(delay = 4000): void {
  if (!_status.signedIn) return;
  void vynl.markSyncDirty().catch(() => {});
  if (_pushTimer) clearTimeout(_pushTimer);
  _pushTimer = setTimeout(() => {
    _pushTimer = null;
    void runCloudSyncPush();
  }, delay);
}

export function flushCloudSyncPush(): void {
  if (_pushTimer !== null) {
    clearTimeout(_pushTimer);
    _pushTimer = null;
  }
  if (!_status.signedIn) return;
  if (_pushing) {
    _pushQueued = true;
    return;
  }
  void runCloudSyncPush();
}

export function retryPendingSync(): void {
  if (!_status.signedIn || !_status.pending || !navigator.onLine) return;
  flushCloudSyncPush();
}

export async function runCloudSyncNow(): Promise<void> {
  if (_pushing || _syncingNow) return;
  _syncingNow = true;
  updateSyncing();
  try {
    await vynl.syncNow();
    await refreshSyncStatus();
  } finally {
    _syncingNow = false;
    updateSyncing();
  }
}
