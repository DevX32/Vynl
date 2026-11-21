let _open = $state(false);
let _path = $state<string | null>(null);

export function getTagEditorOpen(): boolean {
  return _open;
}

export function getTagEditorPath(): string | null {
  return _path;
}

export function openTagEditor(path: string): void {
  _path = path;
  _open = true;
}

export function closeTagEditor(): void {
  _open = false;
  _path = null;
}
