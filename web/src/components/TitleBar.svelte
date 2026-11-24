<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    ArrowLeft,
    ArrowRight,
    Minus,
    Square,
    Copy,
    X,
    Home,
    MessageCircleQuestionMark,
    PanelLeftClose,
    PanelLeftOpen,
    Download,
    LoaderCircle,
  } from "lucide-svelte";
  import { t } from "@lib/i18n";
  import {
    getUpdateAvailable,
    getUpdateInstalling,
    getUpdateStatus,
    installAppUpdate,
  } from "@state/app-update.svelte";

  const win = getCurrentWindow();

  let maximized = $state(false);

  const updateStatus = $derived(getUpdateStatus());
  const updateAvailable = $derived(getUpdateAvailable());
  const updateInstalling = $derived(getUpdateInstalling());
  const updateDownloading = $derived(!!updateStatus?.downloading);
  const updateReady = $derived(!!updateStatus?.ready);
  const showUpdateButton = $derived(
    updateAvailable || updateDownloading || updateReady || updateInstalling,
  );

  let {
    onBack,
    onForward,
    canBack = false,
    canForward = false,
    onHome,
    sidebarOpen = true,
    onToggleSidebar,
    onShowShortcuts,
  }: {
    onBack?: () => void;
    onForward?: () => void;
    canBack?: boolean;
    canForward?: boolean;
    onHome?: () => void;
    sidebarOpen?: boolean;
    onToggleSidebar?: () => void;
    onShowShortcuts?: () => void;
  } = $props();

  onMount(() => {
    void win.isMaximized().then((m) => {
      maximized = m;
    });

    const unlisten = win.onResized(async () => {
      maximized = await win.isMaximized();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  });

  function minimize(): void {
    void win.minimize();
  }

  function toggleMaximize(): void {
    void win.toggleMaximize();
  }

  function close(): void {
    void win.close();
  }

  function onDragDblClick(e: MouseEvent): void {
    if (e.target instanceof HTMLElement && e.target.closest(".controls"))
      return;
    void win.toggleMaximize();
  }
</script>

<div class="titlebar">
  <div class="left" class:collapsed={!sidebarOpen}>
    <button
      class="nav-btn sidebar-toggle"
      aria-label={sidebarOpen ? t("playlists.collapseSidebar") : t("playlists.expandSidebar")}
      onclick={onToggleSidebar}
    >
      {#if sidebarOpen}
        <PanelLeftClose size={14} stroke-width={1.5} />
      {:else}
        <PanelLeftOpen size={14} stroke-width={1.5} />
      {/if}
    </button>
    <button
      class="nav-btn"
      aria-label={t("nav.home")}
      onclick={onHome}
    >
      <Home size={14} stroke-width={1.5} />
    </button>
    <span class="left-divider" aria-hidden="true"></span>
    <button
      class="nav-btn"
      disabled={!canBack || !onBack}
      aria-label={t("common.back")}
      onclick={onBack}
    >
      <ArrowLeft size={14} stroke-width={1.5} />
    </button>
    <button
      class="nav-btn"
      disabled={!canForward || !onForward}
      aria-label={t("titleBar.forward")}
      onclick={onForward}
    >
      <ArrowRight size={14} stroke-width={1.5} />
    </button>
  </div>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="drag-region" data-tauri-drag-region ondblclick={onDragDblClick}>
    <span class="title display" aria-label="Vynl">
      <span class="title-mark" aria-hidden="true">
        <img src="/icon.png" alt="" draggable="false" />
      </span>
    </span>
  </div>

  <div class="controls">
    {#if showUpdateButton}
      <button
        class="update-btn"
        aria-label={t("update.availableTitle")}
        title={t("update.availableTitle")}
        onclick={() => void installAppUpdate()}
        oncontextmenu={(e) => {
          e.preventDefault();
          void installAppUpdate();
        }}
        disabled={updateInstalling || updateDownloading}
      >
        {#if updateInstalling || updateDownloading}
          <span class="spin"><LoaderCircle size={15} stroke-width={1.5} aria-hidden="true" /></span>
        {:else}
          <Download size={15} stroke-width={1.5} />
        {/if}
      </button>
      <span class="controls-divider" aria-hidden="true"></span>
    {/if}
    <button
      class="shortcut-btn"
      aria-label={t("shortcuts.actions.showShortcuts")}
      title={t("shortcuts.actions.showShortcuts")}
      onclick={onShowShortcuts}
    >
      <MessageCircleQuestionMark size={15} stroke-width={1.5} />
    </button>
    <span class="controls-divider" aria-hidden="true"></span>
    <button
      class="ctrl-btn"
      aria-label={t("titleBar.minimize")}
      onclick={minimize}
    >
      <Minus size={14} stroke-width={1.5} />
    </button>
    <button
      class="ctrl-btn"
      aria-label={maximized ? t("titleBar.restore") : t("titleBar.maximize")}
      onclick={toggleMaximize}
    >
      {#if maximized}
        <Copy size={13} stroke-width={1.5} />
      {:else}
        <Square size={12} stroke-width={1.5} />
      {/if}
    </button>
    <button
      class="ctrl-btn close-btn"
      aria-label={t("common.close")}
      onclick={close}
    >
      <X size={15} stroke-width={1.5} />
    </button>
  </div>
</div>

<style>
  .titlebar {
    position: relative;
    display: flex;
    align-items: center;
    height: 32px;
    background: transparent;
    border-bottom: 1px solid var(--line);
    user-select: none;
    flex-shrink: 0;
  }

  .left {
    position: absolute;
    left: 0;
    top: 0;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 2px;
    padding-left: 6px;
    z-index: 2;
    -webkit-app-region: no-drag;
  }

  .nav-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    color: var(--faint);
    transition:
      color 0.1s,
      background 0.1s,
      opacity 0.1s;
  }

  .nav-btn:hover:not(:disabled) {
    color: var(--text);
    background: rgba(255, 255, 255, 0.06);
  }

  .sidebar-toggle {
    margin-right: 130px;
  }

  .left.collapsed .sidebar-toggle {
    margin-right: 20px;
  }

  .left-divider {
    width: 1px;
    height: 14px;
    margin: 0 4px;
    background: var(--line);
  }

  .nav-btn:active:not(:disabled) {
    background: rgba(255, 255, 255, 0.04);
  }

  .nav-btn:disabled {
    opacity: 0.25;
    cursor: default;
  }

  .drag-region {
    flex: 1;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    -webkit-app-region: drag;
  }

  .title {
    position: absolute;
    top: 0;
    left: 50%;
    display: flex;
    height: 100%;
    align-items: center;
    justify-content: center;
    transform: translateX(-50%);
    pointer-events: none;
    user-select: none;
  }

  .title-mark {
    display: flex;
    width: 26px;
    height: 26px;
    align-items: center;
    justify-content: center;
  }

  .title-mark img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .controls {
    position: absolute;
    right: 6px;
    top: 0;
    display: flex;
    align-items: center;
    height: 100%;
    -webkit-app-region: no-drag;
    z-index: 2;
  }

  .update-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    transition:
      color 0.1s,
      background 0.1s;
  }

  .update-btn:hover:not(:disabled) {
    color: var(--text);
    background: color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .update-btn:active:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 28%, transparent);
  }

  .update-btn:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .spin {
    display: inline-flex;
    animation: spin 0.85s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .shortcut-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    color: var(--faint);
    transition:
      color 0.1s,
      background 0.1s;
  }

  .shortcut-btn:hover {
    color: var(--text);
    background: rgba(255, 255, 255, 0.06);
  }

  .shortcut-btn:active {
    background: rgba(255, 255, 255, 0.04);
  }

  .controls-divider {
    width: 1px;
    height: 14px;
    margin: 0 8px 0 6px;
    background: var(--line);
  }

  .ctrl-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 26px;
    border-radius: var(--radius-sm);
    color: var(--faint);
    transition:
      color 0.1s,
      background 0.1s;
  }

  .ctrl-btn:hover {
    color: var(--text);
    background: rgba(255, 255, 255, 0.06);
  }

  .ctrl-btn:active {
    background: rgba(255, 255, 255, 0.04);
  }

  .close-btn:hover {
    background: color-mix(in srgb, var(--red) 14%, transparent);
    color: var(--red);
  }

  .close-btn:active {
    background: color-mix(in srgb, var(--red) 22%, transparent);
    color: var(--red);
  }
</style>
