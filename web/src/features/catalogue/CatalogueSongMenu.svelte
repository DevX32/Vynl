<script lang="ts">
  import { ListMusic } from "lucide-svelte";
  import type { CatalogueTrack } from "@lib/types";
  import {
    getCurrentPlaylists,
    saveStreamToPlaylist,
  } from "@state/playlists.svelte";
  import { toPascalCase } from "@lib/format";
  import { t } from "@lib/i18n";
  import { toasts } from "@lib/toast";
  import ContextMenu, { type CtxEntry } from "@components/ContextMenu.svelte";
  import PlaylistPicker from "@features/vault/PlaylistPicker.svelte";

  let {
    onNavigateAlbum,
    onNavigateArtist,
  }: {
    onNavigateAlbum?: (id: string) => void;
    onNavigateArtist?: (id: string) => void;
  } = $props();

  let ctx = $state<{ x: number; y: number; track: CatalogueTrack } | null>(null);
  let pickerTarget = $state<CatalogueTrack | null>(null);
  let pickerBusy = $state(false);

  const playlists = $derived(getCurrentPlaylists());

  export function open(e: MouseEvent, track: CatalogueTrack): void {
    e.preventDefault();
    e.stopPropagation();
    ctx = { x: e.clientX, y: e.clientY, track };
  }

  function ctxItems(): CtxEntry[] {
    if (!ctx) return [];
    const track = ctx.track;
    const items: CtxEntry[] = [
      {
        label: t("catalogue.addToPlaylist"),
        icon: ListMusic,
        action: () => {
          pickerTarget = track;
        },
      },
    ];

    const albumId = track.album?.id ?? null;
    const artistId = track.artists.find((artist) => artist.id)?.id ?? null;
    if (albumId || artistId) items.push({ type: "separator" });
    if (albumId) {
      items.push({
        label: t("catalogue.goToAlbum"),
        action: () => onNavigateAlbum?.(albumId),
      });
    }
    if (artistId) {
      items.push({
        label: t("catalogue.goToArtist"),
        action: () => onNavigateArtist?.(artistId),
      });
    }
    return items;
  }

  async function addToExisting(playlistId: string): Promise<void> {
    const target = pickerTarget;
    if (!target || pickerBusy) return;
    pickerBusy = true;
    try {
      await saveStreamToPlaylist(target, playlistId);
      const name =
        playlists.find((playlist) => playlist.id === playlistId)?.name ?? "";
      toasts.success(
        t("catalogue.savedToPlaylist", { name: toPascalCase(name) }),
      );
    } catch (error) {
      console.warn("catalogue save to playlist failed:", error);
      toasts.error(t("catalogue.saveFailed"));
    } finally {
      pickerBusy = false;
      pickerTarget = null;
    }
  }

  async function createAndAdd(name: string): Promise<void> {
    const target = pickerTarget;
    if (!target || pickerBusy) return;
    pickerBusy = true;
    try {
      const playlist = await saveStreamToPlaylist(target, null, name);
      toasts.success(
        t("catalogue.savedToPlaylist", {
          name: toPascalCase(playlist.name),
        }),
      );
    } catch (error) {
      console.warn("catalogue create playlist failed:", error);
      toasts.error(t("catalogue.saveFailed"));
    } finally {
      pickerBusy = false;
      pickerTarget = null;
    }
  }
</script>

{#if ctx}
  <ContextMenu
    x={ctx.x}
    y={ctx.y}
    items={ctxItems()}
    onclose={() => (ctx = null)}
  />
{/if}

{#if pickerTarget}
  <PlaylistPicker
    suggestedName={pickerTarget.album?.name ?? pickerTarget.title}
    trackCount={1}
    {playlists}
    busy={pickerBusy}
    onAddExisting={(id) => void addToExisting(id)}
    onCreate={(name) => void createAndAdd(name)}
    onSkip={() => {
      if (pickerBusy) return;
      pickerTarget = null;
    }}
  />
{/if}
