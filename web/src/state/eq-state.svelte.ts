let _open = $state(false);

export function getEqOpen(): boolean {
  return _open;
}

export function closeEq(): void {
  _open = false;
}
