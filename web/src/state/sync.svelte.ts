import { vynl } from "@lib/vynl";
import type { SyncStatus } from "@lib/types";

let _status = $state<SyncStatus>({
  signedIn: false,
  email: null,
  userId: null,
});
let _loaded = $state(false);

export function getSyncStatus(): SyncStatus {
  return _status;
}

export function isSyncStatusLoaded(): boolean {
  return _loaded;
}

export function setSyncStatus(next: SyncStatus): void {
  _status = next;
  _loaded = true;
}

export async function refreshSyncStatus(): Promise<SyncStatus> {
  const next = await vynl.SyncStatus();
  setSyncStatus(next);
  return next;
}

let _pushTimer: ReturnType<typeof setTimeout> | null = null;
let _pushing = false;
let _pushQueued = false;

async function runCloudSyncPush(): Promise<void> {
  if (!_status.signedIn) return;
  if (_pushing) {
    _pushQueued = true;
    return;
  }
  _pushing = true;
  try {
    await vynl.syncPush();
  } catch (e) {
    console.warn("[sync] cloud push failed", e);
  } finally {
    _pushing = false;
    if (_pushQueued) {
      _pushQueued = false;
      void runCloudSyncPush();
    }
  }
}

export function scheduleCloudSyncPush(delay = 4000): void {
  if (!_status.signedIn) return;
  if (_pushTimer) clearTimeout(_pushTimer);
  _pushTimer = setTimeout(() => {
    _pushTimer = null;
    void runCloudSyncPush();
  }, delay);
}
