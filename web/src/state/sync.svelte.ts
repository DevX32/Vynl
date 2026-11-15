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
