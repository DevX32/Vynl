<script lang="ts">
  import {
    SlidersHorizontal as SettingsIcon,
    Disc3,
    Library,
    Music2,
    Link2,
    Plus,
    Compass,
    Heart,
    ChevronUp,
    LogIn,
    LogOut,
    RefreshCw,
    UserRound,
  } from "lucide-svelte";
  import { blobSrc, toPascalCase } from "../lib/format";
  import { blobatarUri } from "blobatar/uri";
  import { t } from "@lib/i18n";
  import { getPlaylistCovers, markCoverFailed, refreshPlaylists } from "@state/playlists.svelte";
  import { getCurrentSettings } from "@state/settings.svelte";
  import {
    getCloudStatus,
    refreshCloudStatus,
    setCloudStatus,
  } from "@state/cloud.svelte";
  import { vynl } from "@lib/vynl";
  import { toasts } from "@lib/toast";
  import { resolveDisplayName } from "@lib/nickname.svelte";
  import { useDragList } from "@lib/drag-list.svelte";
  import { getLikedTracks } from "@state/taste.svelte";
  import type { Page, PlaylistMeta } from "@lib/types";

  let {
    page,
    sidebarOpen,
    playlists,
    selectedId,
    onLibrary,
    onCatalogue,
    onPlayer,
    onVault,
    onSettings,
    onAccount,
    onNewPlaylist,
    onOpenPlaylist,
    onShowCtx,
    onShowRailCtx,
    onReorderPlaylists,
    onOpenLikedSongs,
  }: {
    page: Page;
    sidebarOpen: boolean;
    playlists: PlaylistMeta[];
    selectedId: string | null;
    onLibrary: () => void;
    onCatalogue: () => void;
    onPlayer: () => void;
    onVault: () => void;
    onSettings: () => void;
    onAccount: () => void;
    onNewPlaylist: () => void;
    onOpenPlaylist: (id: string) => void;
    onShowCtx: (e: MouseEvent, id: string, name: string) => void;
    onShowRailCtx: (e: MouseEvent) => void;
    onReorderPlaylists: (fromId: string, toId: string) => void;
    onOpenLikedSongs: () => void;
  } = $props();

  const { drag, start, shouldSkipClick } = useDragList({
    pathAt: (_section, idx) => playlists[idx]?.id,
    onReorder: (fromId, toId) => onReorderPlaylists(fromId, toId),
    rowSelector: () => ".rail-item[data-playlist]",
  });

  const likedTracks = $derived(getLikedTracks());
  const catalogueActive = $derived(
    page === "catalogue" ||
      (page.startsWith("catalogue:") && page !== "catalogue:liked"),
  );

  const cloud = $derived(getCloudStatus());
  const displayName = $derived(
    resolveDisplayName(getCurrentSettings().displayName),
  );
  const avatarSrc = $derived(
    blobatarUri(cloud.signedIn && cloud.email ? cloud.email : displayName, {
      size: 80,
    }),
  );

  let accountMenuOpen = $state(false);
  let accountMenuStyle = $state("");
  let accountWrap: HTMLDivElement | undefined = $state();
  let accountButton: HTMLButtonElement | undefined = $state();

  function toggleAccountMenu(event: MouseEvent): void {
    event.stopPropagation();
    const next = !accountMenuOpen;
    accountMenuOpen = next;
    if (!next) return;
    const rect = accountButton?.getBoundingClientRect();
    if (rect) {
      const width = 220;
      const margin = 12;
      const left = sidebarOpen
        ? rect.left + margin
        : Math.min(rect.right + margin, window.innerWidth - width - margin);
      const bottom = Math.max(4, window.innerHeight - rect.bottom - 8);
      accountMenuStyle = `left: ${left}px; bottom: ${bottom}px;`;
    }
    void refreshCloudStatus().catch(() => {});
  }

  function closeAccountMenu(event: MouseEvent): void {
    if (!accountMenuOpen) return;
    const target = event.target;
    if (accountWrap && target instanceof Node && accountWrap.contains(target)) {
      return;
    }
    accountMenuOpen = false;
  }

  function openAccountModal(): void {
    accountMenuOpen = false;
    onAccount();
  }

  function openAccountSettings(): void {
    accountMenuOpen = false;
    onSettings();
  }

  async function syncCloudNow(): Promise<void> {
    accountMenuOpen = false;
    try {
      const result = await vynl.cloudSync();
      await refreshPlaylists();
      toasts.success(
        result.deleted > 0
          ? t("account.syncDoneDeleted", {
              pulled: result.pulled,
              pushed: result.pushed,
              deleted: result.deleted,
            })
          : t("account.syncDone", {
              pulled: result.pulled,
              pushed: result.pushed,
            }),
        5000,
      );
    } catch (e) {
      toasts.error(
        t("account.syncFailed", {
          error: e instanceof Error ? e.message : String(e),
        }),
        6000,
      );
    }
  }

  async function signOutCloud(): Promise<void> {
    accountMenuOpen = false;
    try {
      await vynl.cloudSignOut();
      setCloudStatus({ signedIn: false, email: null, userId: null });
      toasts.success(t("account.signedOut"), 4000);
    } catch (e) {
      toasts.error(
        t("account.signOutFailed", {
          error: e instanceof Error ? e.message : String(e),
        }),
        6000,
      );
    }
  }

  function onRailPointerDown(e: PointerEvent, idx: number): void {
    const p = playlists[idx];
    if (p) start(e, { idx, path: p.id });
  }
</script>

<svelte:window onclick={closeAccountMenu} />

<nav class="sidebar" class:collapsed={!sidebarOpen} aria-label={t("nav.mainNavigation")}>
  <div class="nav-group">
    <button class="nav-link" class:active={page === "library"} onclick={onLibrary}>
      <span class="nav-icon"><Library size={15} stroke-width={1.5} /></span>
      <span class="nav-label">{t("nav.library")}</span>
    </button>
    <button
      class="nav-link"
      class:active={catalogueActive}
      onclick={onCatalogue}
    >
      <span class="nav-icon"><Compass size={15} stroke-width={1.5} /></span>
      <span class="nav-label">{t("nav.catalogue")}</span>
    </button>
    <button class="nav-link" class:active={page === "player"} onclick={onPlayer}>
      <span class="nav-icon"><Music2 size={15} stroke-width={1.5} /></span>
      <span class="nav-label">{t("nav.nowPlaying")}</span>
    </button>
    <button class="nav-link" class:active={page === "vault"} onclick={onVault}>
      <span class="nav-icon"><Link2 size={15} stroke-width={1.5} /></span>
      <span class="nav-label">{t("nav.vault")}</span>
    </button>
  </div>

  <div class="rail-section">
    <div class="rail-head">
      {#if sidebarOpen}
        <span class="rail-title mono">{t("playlists.title")}</span>
      {/if}
      <button
        class="icon-btn small"
        onclick={onNewPlaylist}
        aria-label={t("playlists.newPlaylist")}
      >
        <Plus size={14} stroke-width={1.5} />
      </button>
    </div>

    <div
      class="rail"
      role="list"
      oncontextmenu={onShowRailCtx}
      bind:this={drag.rowsEl}
    >
      {#if likedTracks.length > 0 || page === "catalogue:liked"}
        <button
          class="rail-item rail-liked"
          class:active={page === "catalogue:liked"}
          onclick={onOpenLikedSongs}
          aria-label={sidebarOpen ? undefined : t("catalogue.likedSongs")}
        >
          <div class="rail-cover liked-cover">
            <Heart size={15} stroke-width={1} />
          </div>

          {#if sidebarOpen}
            <span class="rail-meta">
              <span class="rail-name display">{t("catalogue.likedSongs")}</span>
              <span class="rail-desc mono">
                {`${t("catalogue.playlist")} · ${likedTracks.length} ${
                  likedTracks.length === 1
                    ? t("library.song")
                    : t("library.songsPlural")
                }`}
              </span>
            </span>
          {/if}
        </button>
      {/if}

      {#each playlists as p, i (p.id)}
        {@const covers = getPlaylistCovers(p.id).slice(0, 4)}
        <button
          class="rail-item"
          data-playlist={p.id}
          class:active={page === "playlist" && selectedId === p.id}
          class:dragging={drag.dragging && drag.grabIdx === i}
          class:drag-over={drag.dragging && drag.overIdx === i && drag.grabIdx !== i}
          onclick={() => {
            if (shouldSkipClick()) return;
            onOpenPlaylist(p.id);
          }}
          onpointerdown={(e) => onRailPointerDown(e, i)}
          oncontextmenu={(e) => onShowCtx(e, p.id, p.name)}
          aria-label={sidebarOpen ? undefined : p.name}
        >
          {#if covers.length >= 2}
            <div class="rail-cover-grid">
              {#each covers as c}
                <img
                  class="rail-cover-tile"
                  use:blobSrc={c}
                  alt=""
                  loading="lazy"
                  draggable="false"
                  onerror={() => markCoverFailed(c)}
                />
              {/each}
              {#each Array(4 - covers.length) as _}
                <div class="rail-cover-tile rail-cover-tile-empty"></div>
              {/each}
            </div>
          {:else if covers.length === 1}
            <img
              class="rail-cover"
              use:blobSrc={covers[0]}
              alt=""
              loading="lazy"
              draggable="false"
              onerror={() => markCoverFailed(covers[0])}
            />
          {:else}
            <div class="rail-cover placeholder">
              <Disc3 size={15} stroke-width={1} />
            </div>
          {/if}

          {#if sidebarOpen}
            <span class="rail-meta">
              <span class="rail-name display">{toPascalCase(p.name)}</span>
              <span class="rail-desc mono">
                {`${t("playlists.playlist")} · ${resolveDisplayName(getCurrentSettings().displayName)}`}
              </span>
            </span>
          {/if}
        </button>
      {/each}
      {#if playlists.length === 0 && sidebarOpen}
        <div class="rail-empty">
          <span class="mono">{t("playlists.noPlaylists")}</span>
        </div>
      {/if}
    </div>
  </div>

  <div class="sidebar-foot">
    <div class="account-wrap" bind:this={accountWrap}>
      {#if accountMenuOpen}
        <div class="account-menu" style={accountMenuStyle} role="menu">
          <div class="account-menu-head">
            <span class="menu-name display">{displayName}</span>
            <span class="menu-sub mono">
              {cloud.signedIn ? cloud.email : t("account.notSignedIn")}
            </span>
          </div>
          {#if cloud.signedIn}
            <button
              class="menu-item"
              role="menuitem"
              onclick={openAccountModal}
            >
              <UserRound size={14} stroke-width={1.5} />
              <span>{t("account.menuLabel")}</span>
            </button>
            <button
              class="menu-item"
              role="menuitem"
              onclick={() => void syncCloudNow()}
            >
              <RefreshCw size={14} stroke-width={1.5} />
              <span>{t("account.syncNow")}</span>
            </button>
          {:else}
            <button
              class="menu-item accent"
              role="menuitem"
              onclick={openAccountModal}
            >
              <LogIn size={14} stroke-width={1.5} />
              <span>{t("account.signIn")}</span>
            </button>
          {/if}
          <button class="menu-item" role="menuitem" onclick={openAccountSettings}>
            <SettingsIcon size={14} stroke-width={1.5} />
            <span>{t("nav.settings")}</span>
          </button>
          {#if cloud.signedIn}
            <button
              class="menu-item danger"
              role="menuitem"
              onclick={() => void signOutCloud()}
            >
              <LogOut size={14} stroke-width={1.5} />
              <span>{t("account.signOut")}</span>
            </button>
          {/if}
        </div>
      {/if}
      <button
        class="account-btn"
        bind:this={accountButton}
        onclick={toggleAccountMenu}
        aria-label={cloud.signedIn
          ? `${t("account.signedInAs")} ${cloud.email ?? ""}`
          : t("account.signIn")}
        aria-haspopup="menu"
        aria-expanded={accountMenuOpen}
      >
        <span class="avatar">
          <img src={avatarSrc} alt="" />
          {#if cloud.signedIn}<span class="avatar-dot"></span>{/if}
        </span>
        {#if sidebarOpen}
          <span class="account-meta">
            <span class="account-name">{displayName}</span>
            <span class="account-sub mono">
              {cloud.signedIn ? cloud.email : t("account.notSignedIn")}
            </span>
          </span>
          <span class="account-chevron" class:open={accountMenuOpen}>
            <ChevronUp size={14} stroke-width={1.5} />
          </span>
        {/if}
      </button>
    </div>
  </div>
</nav>

<style>
  .sidebar {
    --cover: 44px;
    --icon: 18px;
    --pad: 6px;
    --collapsed-w: 60px;
    --axis: calc(var(--collapsed-w) / 2);
    --nav-pad-x: calc(var(--axis) - var(--pad) - var(--icon) / 2);
    --rail-pad-x: calc(var(--axis) - var(--pad) - var(--cover) / 2);

    width: 250px;
    flex-shrink: 0;
    padding: 14px var(--pad) 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    transition: width 0.22s ease;
    background: none;
    border-right: 1px solid var(--line);
    position: relative;
    overflow: hidden;
  }

  .sidebar.collapsed {
    width: var(--collapsed-w);
  }

  .sidebar.collapsed .nav-label {
    display: none;
  }

  .sidebar .icon-btn.small {
    width: 26px;
    height: 26px;
    border-radius: 3px;
  }

  .nav-group {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 2px 0 6px;
    border-bottom: 1px solid var(--line);
  }

  .nav-link {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 9px var(--nav-pad-x);
    border-radius: var(--radius-sm);
    color: var(--dim);
    text-align: left;
    transition:
      color 0.15s,
      background 0.15s;
  }

  .nav-link:hover {
    color: var(--text);
    background: var(--bg-raise);
  }

  .nav-link.active {
    color: var(--text);
    background: var(--accent-soft);
  }

  .nav-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--icon);
    flex-shrink: 0;
  }

  .nav-label {
    flex: 1;
    min-width: 0;
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .rail-section {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .rail-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 6px 2px;
  }

  .sidebar.collapsed .rail-head {
    display: none;
  }

  .rail-title {
    font-size: 9.5px;
    letter-spacing: 0.18em;
    color: var(--faint);
    text-transform: uppercase;
  }

  .rail {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .rail::-webkit-scrollbar {
    width: 0;
    height: 0;
  }

  .rail-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 3px var(--rail-pad-x);
    border-radius: var(--radius-sm);
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s;
    user-select: none;
  }

  .rail-item:hover {
    background: var(--bg-raise);
  }

  .rail-item.active {
    background: var(--accent-soft);
  }

  .rail-item.dragging {
    opacity: 0.3;
  }

  .rail-item.drag-over {
    background: transparent;
  }

  .rail-item.drag-over::before {
    content: "";
    position: absolute;
    top: -1px;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--line-strong);
    border-radius: 1px;
  }

  .rail-cover {
    width: var(--cover);
    height: var(--cover);
    border-radius: var(--radius-sm);
    object-fit: cover;
    flex-shrink: 0;
    background: var(--bg-raise);
    box-shadow: var(--shadow-sm);
  }

  .rail-cover.placeholder {
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    border: 1px solid var(--line);
    background: var(--bg-raise);
  }

  .rail-cover.placeholder::before {
    content: "";
    position: absolute;
    inset: -40%;
    background:
      radial-gradient(
        circle at 50% 50%,
        color-mix(in srgb, var(--accent) 52%, transparent) 0%,
        transparent 62%
      );
    filter: blur(10px);
    opacity: 0.85;
  }

  .rail-cover.placeholder :global(svg) {
    position: relative;
    z-index: 1;
    filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.55));
  }

  .rail-cover.liked-cover {
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    border: 1px solid var(--line);
    background: var(--bg-raise);
  }

  .rail-cover.liked-cover::before {
    content: "";
    position: absolute;
    inset: -40%;
    background:
      radial-gradient(
        circle at 50% 50%,
        color-mix(in srgb, var(--accent) 52%, transparent) 0%,
        transparent 62%
      );
    filter: blur(10px);
    opacity: 0.85;
  }

  .rail-cover.liked-cover :global(svg) {
    position: relative;
    z-index: 1;
    filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.55));
  }

  .rail-cover-grid {
    width: var(--cover);
    height: var(--cover);
    flex-shrink: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg);
    box-shadow: var(--shadow-sm);
  }

  .rail-cover-tile {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    background: var(--bg-raise);
  }

  .rail-cover-tile-empty {
    position: relative;
    overflow: hidden;
    background: var(--bg-raise);
  }

  .rail-cover-tile-empty::before {
    content: "";
    position: absolute;
    inset: -40%;
    background:
      radial-gradient(
        circle at 50% 50%,
        color-mix(in srgb, var(--accent) 38%, transparent) 0%,
        transparent 62%
      );
    filter: blur(8px);
    opacity: 0.9;
  }

  .rail-meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .rail-name {
    font-size: 14.5px;
    font-weight: 500;
    line-height: 1.3;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .rail-desc {
    font-size: 10px;
    color: var(--faint);
    letter-spacing: 0.04em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .rail-empty {
    padding: 18px 10px;
    font-size: 10px;
    color: var(--faint);
    letter-spacing: 0.06em;
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: center;
  }

  .sidebar-foot {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 4px;
    padding-top: 6px;
    border-top: 1px solid var(--line);
    margin-top: auto;
  }

  .account-wrap {
    position: relative;
  }

  .account-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 6px var(--nav-pad-x);
    border-radius: var(--radius-sm);
    color: var(--dim);
    text-align: left;
  }

  .sidebar.collapsed .account-btn {
    justify-content: center;
    padding-inline: 0;
  }

  .avatar {
    position: relative;
    width: 32px;
    height: 32px;
    flex-shrink: 0;
    border-radius: 50%;
  }

  .avatar img {
    display: block;
    width: 100%;
    height: 100%;
    border-radius: 50%;
    transform: scale(1.3);
    user-select: none;
  }

  .avatar-dot {
    position: absolute;
    right: -1px;
    bottom: -1px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    border: 2px solid var(--surface);
  }

  .account-meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .account-name {
    font-size: 12.5px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .account-sub {
    font-size: 9.5px;
    color: var(--faint);
    letter-spacing: 0.03em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .account-chevron {
    display: flex;
    flex-shrink: 0;
    color: var(--faint);
    transition: transform 0.15s;
  }

  .account-chevron.open {
    transform: rotate(180deg);
  }

  .account-menu {
    position: fixed;
    z-index: 60;
    width: 220px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    box-shadow: var(--shadow-lg);
  }

  .account-menu-head {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 8px 10px 9px;
    margin-bottom: 2px;
    border-bottom: 1px solid var(--line);
  }

  .menu-name {
    font-size: 14px;
    line-height: 1.2;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .menu-sub {
    font-size: 10px;
    color: var(--faint);
    letter-spacing: 0.03em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    color: var(--dim);
    font-size: 12.5px;
    text-align: left;
    transition:
      background 0.12s,
      color 0.12s;
  }

  .menu-item:hover {
    background: var(--bg-raise);
    color: var(--text);
  }

  .menu-item.accent {
    color: var(--accent);
  }

  .menu-item.danger {
    color: var(--red);
  }

  .menu-item.danger:hover {
    background: color-mix(in srgb, var(--red) 12%, transparent);
    color: var(--red);
  }

  @container player (max-width: 620px) {
    .sidebar {
      --cover: 36px;
      --pad: 4px;
      --collapsed-w: 50px;
      width: 200px;
    }

    .sidebar.collapsed {
      width: var(--collapsed-w);
    }
  }

  @container player (max-width: 420px) {
    .sidebar {
      --cover: 32px;
      --pad: 4px;
      --collapsed-w: 44px;
      width: 180px;
      padding-top: 10px;
      padding-bottom: 8px;
    }

    .sidebar.collapsed {
      width: var(--collapsed-w);
    }

    .nav-link {
      padding-block: 7px;
      gap: 8px;
    }

    .nav-label {
      font-size: 10px;
    }

    .rail-item {
      padding-block: 3px;
      gap: 8px;
    }

    .rail-cover,
    .rail-cover-grid {
      border-radius: 3px;
    }

    .rail-name {
      font-size: 12.5px;
    }
  }
</style>
