import { emit } from "@tauri-apps/api/event";
import type { MiniCommand, MiniSyncState } from "@lib/types";

export const MINI_STATE_EVENT = "vynl:mini:state";
export const MINI_COMMAND_EVENT = "vynl:mini:command";
export const MINI_REQUEST_STATE_EVENT = "vynl:mini:request-state";
export const MINI_CLOSED_EVENT = "vynl:mini:closed";

export function sendMiniState(state: MiniSyncState): void {
  void emit(MINI_STATE_EVENT, state);
}

export function requestMiniState(): void {
  void emit(MINI_REQUEST_STATE_EVENT, null);
}

export function sendMiniCommand(command: MiniCommand): void {
  void emit(MINI_COMMAND_EVENT, command);
}

export function sendMiniClosed(): void {
  void emit(MINI_CLOSED_EVENT, null);
}
