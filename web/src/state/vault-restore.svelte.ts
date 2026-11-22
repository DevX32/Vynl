import type { RestoredBackup } from "@lib/types";

let _pending = $state<RestoredBackup | null>(null);
let _autoStart = $state(false);

export function queueRestore(payload: RestoredBackup, autoStart = false): void {
  _pending = payload;
  _autoStart = autoStart;
}

export function peekRestore(): RestoredBackup | null {
  return _pending;
}

export function takeRestore(): RestoredBackup | null {
  const current = _pending;
  _pending = null;
  return current;
}

export function consumeAutoStart(): boolean {
  const value = _autoStart;
  _autoStart = false;
  return value;
}