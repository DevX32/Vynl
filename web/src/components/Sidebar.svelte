<script lang="ts">
  import {
    SlidersHorizontal as SettingsIcon,
    Disc3,
    Library,
    PanelLeftClose,
    PanelLeftOpen,
    Music2,
    Link2,
    Plus,
    Home,
    Puzzle,
  } from "lucide-svelte";
  import { blobSrc, toPascalCase } from "../lib/format";
  import { t } from "@lib/i18n";
  import { getPlaylistCovers } from "@state/playlists.svelte";
  import { getPluginPages } from "@state/plugins.svelte";
  import { getCurrentSettings } from "@state/settings.svelte";
  import type { Page, PlaylistMeta } from "@lib/types";

  let {
    page,
    sidebarOpen,
    playlists,
    selectedId,
    onSetSidebarOpen,
    onHome,
    onLibrary,
    onPlayer,
    onVault,
    onSettings,
    onPluginPage,
    onNewPlaylist,
    onOpenPlaylist,
    onShowCtx,
    onShowRailCtx,
  }: {
    page: Page;
    sidebarOpen: boolean;
    playlists: PlaylistMeta[];
    selectedId: string | null;
    onSetSidebarOpen: (open: boolean) => void;
    onHome: () => void;
    onLibrary: () => void;
    onPlayer: () => void;
    onVault: () => void;
    onSettings: () => void;
    onPluginPage: (id: string) => void;
    onNewPlaylist: () => void;
    onOpenPlaylist: (id: string) => void;
    onShowCtx: (e: MouseEvent, id: string, name: string) => void;
    onShowRailCtx: (e: MouseEvent) => void;
  } = $props();
</script>

<nav class="sidebar" class:collapsed={!sidebarOpen} aria-label="Main navigation">
  <div class="nav-group">
    <button class="nav-link" class:active={page === "home"} onclick={onHome}>
      <span class="nav-icon"><Home size={15} stroke-width={1.5} /></span>
      <span class="nav-label">{t("nav.home")}</span>
    </button>
    <button class="nav-link" class:active={page === "library"} onclick={onLibrary}>
      <span class="nav-icon"><Library size={15} stroke-width={1.5} /></span>
      <span class="nav-label">{t("nav.library")}</span>
    </button>
    <button class="nav-link" class:active={page === "player"} onclick={onPlayer}>
      <span class="nav-icon"><Music2 size={15} stroke-width={1.5} /></span>
      <span class="nav-label">{t("nav.nowPlaying")}</span>
    </button>
    <button class="nav-link" class:active={page === "vault"} onclick={onVault}>
      <span class="nav-icon"><Link2 size={15} stroke-width={1.5} /></span>
      <span class="nav-label">{t("nav.vault")}</span>
    </button>
    {#each getPluginPages() as pp (pp.pluginId)}
      <button
        class="nav-link"
        class:active={page === `plugin:${pp.pluginId}`}
        onclick={() => onPluginPage(pp.pluginId)}
      >
        <span class="nav-icon"><Puzzle size={15} stroke-width={1.5} /></span>
        <span class="nav-label">{pp.title}</span>
      </button>
    {/each}
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
        title={t("playlists.newPlaylist")}
      >
        <Plus size={14} stroke-width={1.5} />
      </button>
    </div>

    <div class="rail" role="list" oncontextmenu={onShowRailCtx}>
      {#each playlists as p (p.id)}
        {@const covers = getPlaylistCovers(p.id).slice(0, 4)}
        <button
          class="rail-item"
          class:active={page === "playlist" && selectedId === p.id}
          onclick={() => onOpenPlaylist(p.id)}
          oncontextmenu={(e) => onShowCtx(e, p.id, p.name)}
          title={sidebarOpen ? undefined : p.name}
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
                {getCurrentSettings().displayName
                  ? `${t("playlists.playlist")} · ${getCurrentSettings().displayName}`
                  : t("playlists.playlist")}
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
    <button class="nav-link" class:active={page === "settings"} onclick={onSettings}>
      <span class="nav-icon"><SettingsIcon size={15} stroke-width={1.5} /></span>
      <span class="nav-label">{t("nav.settings")}</span>
    </button>
    <button class="nav-link collapse-btn" onclick={() => onSetSidebarOpen(!sidebarOpen)}>
      <span class="nav-icon">
        {#if sidebarOpen}
          <PanelLeftClose size={15} stroke-width={1.5} />
        {:else}
          <PanelLeftOpen size={15} stroke-width={1.5} />
        {/if}
      </span>
      <span class="nav-label">
        {sidebarOpen ? t("playlists.collapseSidebar") : t("playlists.expandSidebar")}
      </span>
    </button>
  </div>
</nav>

<style>
  .sidebar {
    --cover: 40px;
    --icon: 18px;
    --pad: 6px;
    --collapsed-w: 60px;
    --axis: calc(var(--collapsed-w) / 2);
    --nav-pad-x: calc(var(--axis) - var(--pad) - var(--icon) / 2);
    --rail-pad-x: calc(var(--axis) - var(--pad) - var(--cover) / 2);

    width: 252px;
    flex-shrink: 0;
    padding: 14px var(--pad) 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    transition: width 0.22s ease;
    background: none;
    position: relative;
    overflow: hidden;
  }

  .sidebar.collapsed {
    width: var(--collapsed-w);
  }

  .sidebar.collapsed .nav-label {
    display: none;
  }

  .sidebar.collapsed .rail {
    padding-right: 0;
    scrollbar-width: none;
  }

  .sidebar.collapsed .rail::-webkit-scrollbar {
    display: none;
  }

  .collapse-btn {
    color: var(--faint);
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
    scrollbar-width: thin;
    scrollbar-color: rgba(240, 240, 236, 0.12) transparent;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-right: 2px;
  }

  .rail::-webkit-scrollbar {
    width: 6px;
  }

  .rail::-webkit-scrollbar-thumb {
    background: rgba(240, 240, 236, 0.12);
    border-radius: 3px;
  }

  .rail-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 6px var(--rail-pad-x);
    border-radius: var(--radius-sm);
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s;
  }

  .rail-item:hover {
    background: var(--bg-raise);
  }

  .rail-item.active {
    background: var(--accent-soft);
  }

  .rail-cover {
    width: var(--cover);
    height: var(--cover);
    border-radius: 4px;
    object-fit: cover;
    flex-shrink: 0;
    background: var(--bg-raise);
    box-shadow: var(--shadow-sm);
  }

  .rail-cover.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--faint);
    border: 1px solid var(--line);
    background: var(--placeholder-gradient);
  }

  .rail-cover-grid {
    width: var(--cover);
    height: var(--cover);
    flex-shrink: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0;
    border-radius: 4px;
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
    background: var(--placeholder-gradient);
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
      padding-block: 6px;
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