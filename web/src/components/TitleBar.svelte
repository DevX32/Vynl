<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { ArrowLeft, ArrowRight, Minus, Square, Copy, X } from "lucide-svelte";
  import { t } from "@lib/i18n";

  const win = getCurrentWindow();

  let maximized = $state(false);

  let {
    onBack,
    onForward,
    canBack = false,
    canForward = false,
  }: {
    onBack?: () => void;
    onForward?: () => void;
    canBack?: boolean;
    canForward?: boolean;
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
  <div class="left">
    <button
      class="nav-btn"
      disabled={!canBack || !onBack}
      aria-label={t("titleBar.back")}
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
      aria-label={t("titleBar.close")}
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
    background: var(--bg);
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
    border-radius: 3px;
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
    right: 0;
    top: 0;
    display: flex;
    align-items: center;
    height: 100%;
    -webkit-app-region: no-drag;
    z-index: 2;
  }

  .ctrl-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 100%;
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
    background: var(--red);
    color: #fff;
  }
</style>
