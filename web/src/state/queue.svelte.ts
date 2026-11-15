const QUEUE_OPEN_KEY = "vynl.queueOpen";

function load(): boolean {
  if (typeof localStorage === "undefined") return false;
  return localStorage.getItem(QUEUE_OPEN_KEY) === "true";
}

let open = $state(load());

export function getQueueOpen(): boolean {
  return open;
}

export function setQueueOpen(value: boolean): void {
  open = value;
  try {
    localStorage.setItem(QUEUE_OPEN_KEY, String(value));
  } catch {}
}

export function toggleQueue(): void {
  setQueueOpen(!open);
}
