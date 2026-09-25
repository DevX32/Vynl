let _open = $state(false);

export function getEqOpen(): boolean {
  return _open;
}

export function openEq(): void {
  _open = true;
}

export function closeEq(): void {
  _open = false;
}

export function toggleEq(): void {
  _open = !_open;
}
