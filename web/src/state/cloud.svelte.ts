import { vynl } from "@lib/vynl";
import type { CloudStatus } from "@lib/types";

let _status = $state<CloudStatus>({
  signedIn: false,
  email: null,
  userId: null,
});
let _loaded = $state(false);

export function getCloudStatus(): CloudStatus {
  return _status;
}

export function isCloudStatusLoaded(): boolean {
  return _loaded;
}

export function setCloudStatus(next: CloudStatus): void {
  _status = next;
  _loaded = true;
}

export async function refreshCloudStatus(): Promise<CloudStatus> {
  const next = await vynl.cloudStatus();
  setCloudStatus(next);
  return next;
}
