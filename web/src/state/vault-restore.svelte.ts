import type { RestoredBackup } from "@lib/types";

let _pending = $state<RestoredBackup | null>(null);

export function queueRestore(payload: RestoredBackup): void {
  _pending = payload;
}

export function peekRestore(): RestoredBackup | null {
  return _pending;
}

export function takeRestore(): RestoredBackup | null {
  const current = _pending;
  _pending = null;
  return current;
}