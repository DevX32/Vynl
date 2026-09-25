<script lang="ts">
  import { Copy, Settings2, X, Download } from "lucide-svelte";
  import { fly, fade } from "svelte/transition";
  import { cubicOut, cubicIn } from "svelte/easing";
  import { t } from "@lib/i18n";
  import { vynl } from "../../lib/vynl";
  import { blobSrc } from "../../lib/format";
  import {
    getCurrentTrack,
    getCurrentId,
    getIsTrackPlaying,
    seekTo,
  } from "@state/now-playing.svelte";
  import {
    loadLyrics,
    getLyricsLoading,
    getLyricsResult,
    getLyricsLines,
    getLyricsSynced,
    getLyricsActiveIndex,
    getLyricsActiveWordIndex,
    isInstrumental,
  } from "@state/lyrics.svelte";
  import LyricsSearchOverlay from "./LyricsSearchOverlay.svelte";
  import LyricsPasteOverlay from "./LyricsPasteOverlay.svelte";
  import LyricsHelpOverlay from "./LyricsHelpOverlay.svelte";

  let { onclose, fullscreen = false }: { onclose: () => void; fullscreen?: boolean } = $props();

  let lineEls = $state<HTMLElement[]>([]);
  let container = $state<HTMLDivElement>();
  let settingsEl = $state<HTMLDivElement>();

  let autoScroll = $state(readAutoScrollPref());
  let selected = $state<number[]>([]);
  let settingsOpen = $state(false);

  type Overlay = "help" | "search" | "paste" | null;
  let overlay = $state<Overlay>(null);

  let toast = $state("");
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

  const np = $derived(getCurrentTrack());
  const selectedCharCount = $derived(
    selected.reduce((sum, i) => sum + (getLyricsLines()[i]?.text?.length ?? 0), 0),
  );

  function readAutoScrollPref(): boolean {
    if (typeof localStorage === "undefined") return true;
    return localStorage.getItem("vynl.autoscroll") !== "false";
  }

  function setAutoScroll(value: boolean): void {
    autoScroll = value;
    localStorage.setItem("vynl.autoscroll", String(value));
  }

  function showToast(message: string): void {
    toast = message;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = ""), 1600);
  }

  function copyText(text: string): void {
    vynl.copyText(text);
    showToast(t("lyrics.copied"));
  }

  function closeOverlay(): void {
    overlay = null;
  }

  function openOverlay(next: Exclude<Overlay, null>): void {
    overlay = next;
    settingsOpen = false;
  }

  $effect(() => {
    return () => {
      if (toastTimer) clearTimeout(toastTimer);
    };
  });

  $effect(() => {
    const id = getCurrentId();
    loadLyrics(id);
    selected = [];
    overlay = null;
  });

  $effect(() => {
    if (!getIsTrackPlaying() || !autoScroll) return;
    const activeIndex = getLyricsActiveIndex();
    if (activeIndex < 0) return;
    const el = lineEls[activeIndex];
    const c = container;
    if (!el || !c) return;
    const target = el.offsetTop - c.clientHeight / 2 + el.clientHeight / 2;
    if (Math.abs(c.scrollTop - target) > 8) {
      c.scrollTo({ top: target, behavior: "smooth" });
    }
  });

  $effect(() => {
    if (!settingsOpen) return;
    const onPointerDown = (e: PointerEvent): void => {
      const target = e.target as HTMLElement;
      if (settingsEl?.contains(target) || target.closest(".lyr-icon")) return;
      settingsOpen = false;
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  });

  $effect(() => {
    const onKeyDown = (e: KeyboardEvent): void => {
      if (e.key !== "Escape") return;
      if (overlay) {
        closeOverlay();
      } else if (settingsOpen) {
        settingsOpen = false;
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  });

  function toggleLine(i: number): void {
    selected = selected.includes(i)
      ? selected.filter((x) => x !== i)
      : [...selected, i].sort((a, b) => a - b);
  }

  function onLineClick(e: MouseEvent, i: number): void {
    e.stopPropagation();

    if (e.ctrlKey || e.metaKey) {
      toggleLine(i);
      return;
    }
    const line = getLyricsLines()[i];
    if (getLyricsSynced() && line?.time >= 0) {
      seekTo(line.time);
      return;
    }
    selected = selected.length === 1 && selected[0] === i ? [] : [i];
  }

  function onLineContextMenu(e: MouseEvent, i: number): void {
    const text = getLyricsLines()[i]?.text;
    if (!text) return;
    e.preventDefault();
    copyText(text);
  }

  function clearSelectionOnBackground(e: PointerEvent): void {
    const target = e.target as HTMLElement;
    if (target.closest(".lyr-line") || e.ctrlKey || e.metaKey) return;
    selected = [];
  }

  function copyAllLyrics(): void {
    const lines = getLyricsLines()
      .map((l) => l.text)
      .filter(Boolean)
      .join("\n");
    if (!lines) return;
    copyText(lines);
    showToast(t("lyrics.copiedAll", { n: getLyricsLines().length }));
  }

  function copySelectedLyrics(): void {
    const lines = getLyricsLines();
    const joined = selected
      .map((i) => lines[i]?.text)
      .filter((line): line is string => !!line)
      .join("\n");
    if (!joined) return;
    copyText(joined);
    showToast(t("lyrics.copiedN", { n: selected.length }));
  }

  function openSearch(): void {
    openOverlay("search");
  }

  function openPaste(): void {
    openOverlay("paste");
  }

  async function exportLyrics(): Promise<void> {
    const result = getLyricsResult();
    const track = getCurrentTrack();
    if (!result || !track) return;
    const ext = getLyricsSynced() ? "lrc" : "txt";
    const savedPath = await vynl.lyricsExport({
      content: result.text,
      defaultName: `${track.artist} - ${track.title}.${ext}`,
    });
    if (savedPath) {
      showToast(t("lyrics.savedTo", { file: savedPath.split(/[/\\]/).pop() ?? savedPath }));
    }
  }
</script>

{#snippet modalShell(title: string, close: () => void, body: import("svelte").Snippet)}
  <div class="modal" transition:fade={{ duration: 120 }}>
    <button class="modal-backdrop" onclick={close} aria-label={t("lyrics.helpEsc")}></button>
    <div
      class="modal-content"
      in:fly={{ y: 8, duration: 200, easing: cubicOut }}
      out:fly={{ y: 8, duration: 150, easing: cubicIn }}
    >
      <div class="modal-title display">{title}</div>
      {@render body()}
    </div>
  </div>
{/snippet}

<section class="lyrics-page">
  {#if np}
    {#if !fullscreen}
      <header class="lyr-head">
        <div class="lyr-track">
          {#if np.cover}
            <img class="lyr-cover" use:blobSrc={np.cover} alt="" draggable="false" />
          {:else}
            <div class="lyr-cover placeholder">{np.title.charAt(0).toUpperCase()}</div>
          {/if}
          <div class="lyr-track-text">
            <div class="lyr-title">{np.title}</div>
            <div class="lyr-artist">{np.artist || t("lyrics.unknownArtist")}</div>
            {#if np.album}<div class="lyr-album">{np.album}</div>{/if}
          </div>
        </div>

        <div class="lyr-controls">
          {#if getLyricsResult()}
            <button class="lyr-icon" onclick={exportLyrics} aria-label={t("lyrics.exportLyrics")}>
              <Download size={15} stroke-width={1.5} />
            </button>
          {/if}
          <button class="lyr-icon" onclick={copyAllLyrics} aria-label={t("lyrics.copyAllLyrics")}>
            <Copy size={15} stroke-width={1.5} />
          </button>
          <button
            class="lyr-icon"
            class:active={settingsOpen}
            onclick={() => (settingsOpen = !settingsOpen)}
            aria-label={t("lyrics.settings")}
          >
            <Settings2 size={15} stroke-width={1.5} />
          </button>
          <button class="lyr-icon" onclick={onclose} aria-label={t("lyrics.closeLyrics")}>
            <X size={15} stroke-width={1.5} />
          </button>
        </div>

        {#if settingsOpen}
          <div class="lyr-settings" transition:fly={{ y: -6, duration: 150, easing: cubicOut }} bind:this={settingsEl}>
            <div class="settings-row">
              <span class="settings-label mono">{t("lyrics.autoScroll")}</span>
              <button
                class="mini-switch"
                class:on={autoScroll}
                onclick={() => setAutoScroll(!autoScroll)}
                aria-label={t("lyrics.toggleAutoScroll")}
              ><span></span></button>
            </div>
            <div class="settings-row">
              <button class="btn-ghost settings-action mono" onclick={openSearch}>{t("lyrics.search")}</button>
            </div>
            <div class="settings-row">
              <button class="btn-ghost settings-action mono" onclick={openPaste}>{t("lyrics.paste")}</button>
            </div>
            <div class="settings-row">
              <button class="btn-ghost settings-action mono" onclick={() => openOverlay("help")}>
                {t("lyrics.shortcuts")}
              </button>
            </div>
          </div>
        {/if}
      </header>
    {/if}

    <div
      class="lyrics-scroll"
      role="presentation"
      class:paused={!getIsTrackPlaying()}
      bind:this={container}
      onpointerdown={clearSelectionOnBackground}
    >
      {#if getLyricsLoading() && getLyricsLines().length === 0}
        <div class="lyr-empty" transition:fade={{ duration: 120 }}>
          <div class="lyr-empty-sub mono">{t("lyrics.loadingLyrics")}</div>
        </div>
      {:else if !getLyricsResult()}
        <div class="lyr-empty" transition:fade={{ duration: 120 }}>
          <div class="lyr-empty-title display">{t("lyrics.noLyricsFound")}</div>
        </div>
      {:else if getLyricsLines().length === 0}
        <div class="lyr-empty mono" transition:fade={{ duration: 120 }}>
          {t("lyrics.emptyLyricsFile")}
        </div>
      {:else}
        {#each getLyricsLines() as line, i (i)}
          {@const activeIndex = getLyricsActiveIndex()}
          {@const synced = getLyricsSynced()}
          {@const activeWordIndex = getLyricsActiveWordIndex()}
          <button
            type="button"
            class="lyr-line"
            class:active={i === activeIndex}
            class:before={synced && activeIndex >= 0 && i < activeIndex}
            class:selected={selected.includes(i)}
            class:seekable={synced && line.time >= 0}
            bind:this={lineEls[i]}
            onclick={(e) => onLineClick(e, i)}
            oncontextmenu={(e) => onLineContextMenu(e, i)}
          >
            {#if synced && isInstrumental(line.text)}
              <span class="lyr-dots" class:on={i === activeIndex}>
                <span></span><span></span><span></span>
              </span>
            {:else if i === activeIndex && line.words?.length}
              <span class="lyr-text karaoke">
                {#each line.words as word, wordIndex (wordIndex)}
                  <span class:karaoke-active={wordIndex <= activeWordIndex}>{word.text}{" "}</span>
                {/each}
              </span>
            {:else}
              <span class="lyr-text">{line.text || "\u00A0"}</span>
            {/if}
          </button>
        {/each}
      {/if}
    </div>

    {#if selected.length > 0}
      <div class="selection-bar" transition:fly={{ y: 8, duration: 160, easing: cubicOut }}>
        <span class="selection-count mono">
          {t("lyrics.nLines", { n: selected.length, s: selected.length > 1 ? "s" : "", c: selectedCharCount })}
        </span>
        <button class="sel-btn" onclick={copySelectedLyrics} aria-label={t("lyrics.copySelected")}>
          <Copy size={13} stroke-width={1.5} />
        </button>
        <button class="sel-btn" onclick={() => (selected = [])} aria-label={t("lyrics.clearSelection")}>
          <X size={13} stroke-width={1.5} />
        </button>
      </div>
    {/if}

    {#if overlay === "help"}{@render modalShell(t("lyrics.helpTitle"), closeOverlay, helpBody)}{/if}
    {#if overlay === "search"}{@render modalShell(t("lyrics.searchLyrics"), closeOverlay, searchBody)}{/if}
    {#if overlay === "paste"}{@render modalShell(t("lyrics.pasteLyrics"), closeOverlay, pasteBody)}{/if}

    {#if toast}
      <div class="lyr-toast mono" transition:fade={{ duration: 120 }}>{toast}</div>
    {/if}
  {:else}
    <div class="lyr-empty">
      <div class="lyr-empty-title display">{t("player.nothingPlaying")}</div>
      <div class="lyr-empty-sub mono">{t("lyrics.playToSee")}</div>
    </div>
  {/if}
</section>

{#snippet helpBody()}
  <LyricsHelpOverlay onClose={closeOverlay} />
{/snippet}

{#snippet searchBody()}
  <LyricsSearchOverlay onClose={closeOverlay} />
{/snippet}

{#snippet pasteBody()}
  <LyricsPasteOverlay onClose={closeOverlay} />
{/snippet}

<style>
  :root {
    --ease-out: cubic-bezier(0.22, 1, 0.36, 1);
  }

  .lyrics-page {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 20px;
    padding: 20px 0;
  }

  .lyr-head {
    position: relative;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--line);
  }
  .lyr-track {
    display: flex;
    gap: 14px;
    min-width: 0;
    align-items: center;
  }
  .lyr-cover {
    width: 58px;
    height: 58px;
    border-radius: var(--radius-sm);
    object-fit: cover;
    flex-shrink: 0;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.35);
  }
  .lyr-cover.placeholder {
    background: var(--placeholder-gradient);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 20px;
  }
  .lyr-track-text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 3px;
  }
  .lyr-title {
    font-size: 20px;
    font-weight: 600;
    line-height: 1.2;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text);
  }
  .lyr-artist {
    font-size: 14px;
    font-weight: 500;
    letter-spacing: 0.04em;
    color: var(--dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .lyr-album {
    font-size: 11px;
    color: var(--faint);
    letter-spacing: 0.03em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-top: 1px;
  }
  .lyr-controls {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .lyr-icon {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(16, 16, 20, 0.45);
    border: 1px solid var(--line);
    color: var(--faint);
    padding: 0;
    border-radius: 6px;
    cursor: pointer;
    transform: rotate(45deg);
    transition: color 0.15s, background 0.15s, border-color 0.15s;
  }
  .lyr-icon :global(svg) {
    transform: rotate(-45deg);
  }
  .lyr-icon:hover {
    color: var(--dim);
    background: var(--bg-raise);
    border-color: var(--line-strong);
  }
  .lyr-icon.active {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
    background: var(--accent-soft);
  }

  .lyr-settings {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    z-index: 30;
    min-width: 180px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .settings-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 6px 8px;
    border-radius: 5px;
    transition: background 0.12s;
  }
  .settings-row:hover {
    background: var(--bg);
  }
  .settings-label {
    font-size: 10.5px;
    letter-spacing: 0.05em;
    color: var(--dim);
  }
  .settings-action {
    padding: 2px 0;
  }

  .mini-switch {
    width: 28px;
    height: 15px;
    border-radius: 3px;
    background: var(--line-strong);
    border: none;
    padding: 0;
    cursor: pointer;
    position: relative;
    transition: background 0.15s;
  }
  .mini-switch span {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 11px;
    height: 11px;
    border-radius: 2px;
    background: var(--faint);
    transition: transform 0.15s var(--ease-out), background 0.15s;
  }
  .mini-switch.on {
    background: var(--accent-soft);
  }
  .mini-switch.on span {
    transform: translateX(13px);
    background: var(--accent);
  }

  .btn-ghost {
    background: none;
    border: none;
    color: var(--dim);
    font-size: 11px;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: color 0.15s;
  }
  .btn-ghost:hover {
    color: var(--accent);
  }
  .lyrics-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: none;
    scroll-behavior: smooth;
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 24px 0 80px;
    mask-image: linear-gradient(to bottom, transparent, black 56px, black calc(100% - 56px), transparent);
    -webkit-mask-image: linear-gradient(to bottom, transparent, black 56px, black calc(100% - 56px), transparent);
  }
  .lyrics-scroll::-webkit-scrollbar {
    display: none;
  }

  .lyr-empty {
    margin: auto;
    text-align: center;
    color: var(--faint);
    font-size: 11.5px;
  }
  .lyr-empty-title {
    font-size: 20px;
    color: var(--dim);
    margin-bottom: 6px;
  }
  .lyr-empty-sub {
    font-size: 11px;
  }

  .lyr-line {
    font-size: 18px;
    line-height: 1.65;
    color: var(--faint);
    padding: 6px 16px;
    flex-shrink: 0;
    position: relative;
    display: block;
    width: 100%;
    max-width: 720px;
    margin: 0 auto;
    text-align: center;
    background: none;
    border: none;
    cursor: pointer;
    overflow-wrap: break-word;
    transition:
      color 0.5s cubic-bezier(0.22, 1, 0.36, 1),
      opacity 0.5s cubic-bezier(0.22, 1, 0.36, 1),
      transform 0.55s cubic-bezier(0.34, 1.3, 0.36, 1),
      background 0.25s ease,
      text-shadow 0.5s cubic-bezier(0.22, 1, 0.36, 1),
      filter 0.5s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .lyr-line.seekable {
    cursor: pointer;
  }
  .lyr-line.active {
    color: var(--text);
    font-weight: 500;
    transform: scale(1.15);
    transform-origin: center;
    text-shadow: 0 0 18px var(--accent-glow), 0 0 40px rgba(185, 167, 255, 0.15);
  }
  .lyr-line.before {
    opacity: 0.35;
    filter: blur(0.3px);
  }
  .lyr-line.selected {
    color: var(--accent);
    padding-left: 14px;
  }

  .lyr-text.karaoke {
    display: inline;
  }
  .lyr-text.karaoke > span {
    display: inline;
    transition: color 0.15s ease, transform 0.15s ease;
    color: var(--faint);
  }
  .lyr-text.karaoke > span.karaoke-active {
    color: var(--accent);
    transform: scale(1.03);
  }

  .lyr-dots {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    opacity: 0.3;
    transition: opacity 0.2s ease;
  }
  .lyr-dots.on {
    opacity: 1;
  }
  .lyr-dots span {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: currentColor;
  }
  .lyr-dots.on span {
    animation: lyr-dot-wave 0.7s ease-in-out infinite;
  }
  .lyr-dots.on span:nth-child(2) {
    animation-delay: 0.15s;
  }
  .lyr-dots.on span:nth-child(3) {
    animation-delay: 0.3s;
  }
  .lyrics-scroll.paused .lyr-dots.on span {
    animation-play-state: paused;
  }
  @keyframes lyr-dot-wave {
    0%, 100% {
      transform: translateY(0);
      opacity: 0.4;
    }
    50% {
      transform: translateY(-5px);
      opacity: 1;
    }
  }

  .selection-bar {
    position: absolute;
    bottom: 12px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 30;
    display: flex;
    align-items: center;
    gap: 4px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 5px 6px 5px 16px;
  }
  .selection-count {
    font-size: 10px;
    color: var(--dim);
    letter-spacing: 0.05em;
    padding-right: 6px;
  }
  .sel-btn {
    background: none;
    border: none;
    color: var(--faint);
    padding: 5px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: color 0.15s;
  }
  .sel-btn:hover {
    color: var(--text);
  }
  .modal {
    position: absolute;
    inset: 0;
    z-index: 40;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .modal-backdrop {
    position: fixed;
    inset: 0;
    border: none;
    cursor: pointer;
    background: rgba(0, 0, 0, 0.3);
    backdrop-filter: blur(12px);
  }
  .modal-content {
    position: relative;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-lg);
    padding: 24px 28px;
    min-width: 360px;
    max-width: 460px;
    max-height: 80%;
    display: flex;
    flex-direction: column;
    gap: 14px;
    overflow: hidden;
  }
  .modal-title {
    font-size: 17px;
  }

  .lyr-toast {
    position: absolute;
    bottom: 56px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 50;
    background: rgba(24, 24, 28, 0.92);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 8px 18px;
    font-size: 10.5px;
    letter-spacing: 0.05em;
    color: var(--text);
  }

  @media (prefers-reduced-motion: reduce) {
    .lyr-line,
    .lyr-text.karaoke > span,
    .lyr-icon,
    .mini-switch span {
      transition-duration: 0.01ms !important;
    }
    .lyr-line.active {
      text-shadow: none !important;
    }
    .lyr-line.before {
      filter: none !important;
    }
    .lyr-dots.on span {
      animation: none;
      opacity: 1;
    }
  }
</style>