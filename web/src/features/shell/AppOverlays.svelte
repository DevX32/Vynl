<script lang="ts">
  import FullscreenPlayer from "../player/FullscreenPlayer.svelte";
  import ShortcutsOverlay from "../../components/ShortcutsOverlay.svelte";
  import AccountModal from "../account/AccountModal.svelte";
  import TagEditorModal from "../../components/TagEditorModal.svelte";
  import MobileSyncModal from "../settings/MobileSyncModal.svelte";
  import ContextMenu from "../../components/ContextMenu.svelte";
  import Toast from "../../components/Toast.svelte";
  import PlaylistModals from "../playlists/PlaylistModals.svelte";
  import MoveToVynlDialog from "../playlists/MoveToVynlDialog.svelte";
  import { getShowShortcutsOverlay } from "@state/shortcuts.svelte";
  import { getTagEditorOpen } from "@state/tag-editor.svelte";
  import { getMobileSyncUiOpen } from "@state/mobile-sync.svelte";
  import { getCtxMenu, closeCtxMenu, ctxItems } from "@lib/app-ctx.svelte";
  import type { StreamImportResult } from "@lib/types";
  import type { Page } from "@lib/types";

  let {
    showFullscreen = $bindable(),
    accountOpen = $bindable(),
    moveToVynlOpen = $bindable(),
    createTarget = $bindable(),
    editTarget = $bindable(),
    onNavigate,
    onMovedToVynl,
    onNewPlaylist,
  }: {
    showFullscreen: boolean;
    accountOpen: boolean;
    moveToVynlOpen: boolean;
    createTarget: boolean;
    editTarget: { id: string; name: string; cover: string | null } | null;
    onNavigate: (p: Page) => void;
    onMovedToVynl: (result: StreamImportResult) => void;
    onNewPlaylist: () => void;
  } = $props();
</script>

{#if showFullscreen}
  <FullscreenPlayer onclose={() => (showFullscreen = false)} />
{/if}

{#if getShowShortcutsOverlay()}
  <ShortcutsOverlay />
{/if}

{#if accountOpen}
  <AccountModal onclose={() => (accountOpen = false)} />
{/if}

{#if getTagEditorOpen()}
  <TagEditorModal />
{/if}

{#if getMobileSyncUiOpen()}
  <MobileSyncModal />
{/if}

{#if getCtxMenu()}
  {@const menu = getCtxMenu()!}
  <ContextMenu
    x={menu.x}
    y={menu.y}
    onClose={closeCtxMenu}
    onNewPlaylist={menu.type === "rail" ? onNewPlaylist : undefined}
    onMoveToVynl={menu.type === "rail" ? () => (moveToVynlOpen = true) : undefined}
    items={ctxItems((id, name, cover) => (editTarget = { id, name, cover }))}
  />
{/if}

<PlaylistModals bind:createOpen={createTarget} bind:editTarget onnavigate={onNavigate} />

{#if moveToVynlOpen}
  <MoveToVynlDialog onClose={() => (moveToVynlOpen = false)} onImported={onMovedToVynl} />
{/if}

<Toast />
