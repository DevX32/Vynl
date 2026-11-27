import { t } from "@lib/i18n";
import { getPlaylistCustomCover } from "@state/playlists.svelte";
import { Pencil } from "lucide-svelte";
import type { CtxEntry } from "../components/ContextMenu.svelte";

export type CtxMenuState = {
  x: number;
  y: number;
  playlistId?: string;
  playlistName?: string;
  type: "playlist" | "rail";
} | null;

let ctxMenu = $state<CtxMenuState>(null);

export function getCtxMenu(): CtxMenuState { return ctxMenu; }
export function closeCtxMenu(): void { ctxMenu = null; }

export function showCtx(e: MouseEvent, id: string, name: string): void {
  e.preventDefault();
  e.stopPropagation();
  ctxMenu = { x: e.clientX, y: e.clientY, playlistId: id, playlistName: name, type: "playlist" };
}

export function showRailCtx(e: MouseEvent): void {
  if ((e.target as HTMLElement).closest(".rail-item")) return;
  e.preventDefault();
  ctxMenu = { x: e.clientX, y: e.clientY, type: "rail" };
}

export function ctxItems(onEdit: (id: string, name: string, cover: string | null) => void): CtxEntry[] {
  const m = ctxMenu;
  if (!m || m.type !== "playlist" || !m.playlistId || !m.playlistName) return [];
  const id = m.playlistId;
  const name = m.playlistName;
  return [
    {
      label: t("playlist.editPlaylist"),
      icon: Pencil,
      action: () => { onEdit(id, name, getPlaylistCustomCover(id) ?? null); },
    },
  ];
}
