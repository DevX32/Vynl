import type { Page } from "@lib/types";

function loadSidebarOpen(): boolean {
  if (typeof localStorage === "undefined") return true;
  return localStorage.getItem("vynl.sidebarOpen") !== "false";
}

export const nav = $state({
  page: "home" as Page,
  history: ["home"] as Page[],
  historyIdx: 0,
  sidebarOpen: loadSidebarOpen(),
});

export function navigate(p: Page): void {
  if (nav.history[nav.historyIdx] === p) return;
  nav.history.splice(nav.historyIdx + 1);
  nav.history.push(p);
  nav.historyIdx = nav.history.length - 1;
  nav.page = p;
}

export function goBack(): void {
  if (nav.historyIdx <= 0) return;
  nav.historyIdx--;
  nav.page = nav.history[nav.historyIdx];
}

export function goForward(): void {
  if (nav.historyIdx >= nav.history.length - 1) return;
  nav.historyIdx++;
  nav.page = nav.history[nav.historyIdx];
}

export function setSidebarOpen(open: boolean): void {
  nav.sidebarOpen = open;
  if (typeof localStorage !== "undefined") localStorage.setItem("vynl.sidebarOpen", String(open));
}
