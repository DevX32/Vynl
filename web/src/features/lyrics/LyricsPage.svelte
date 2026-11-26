<script lang="ts">
  import { ClipboardPaste, Copy, Keyboard, Search, Settings2, X } from "lucide-svelte";
  import { cubicIn, cubicOut } from "svelte/easing";
  import { fade, fly } from "svelte/transition";
  import { t } from "@lib/i18n";
  import { vynl } from "@lib/vynl";
  import { blobSrc } from "@lib/format";
  import {
    getCurrentId,
    getCurrentTrack,
    getIsTrackPlaying,
    seekTo,
  } from "@state/now-playing.svelte";
  import {
    getLyricsActiveIndex,
    getLyricsError,
    getLyricsLines,
    getLyricsLoading,
    getLyricsResult,
    getLyricsSynced,
    isInstrumental,
    loadLyrics,
  } from "@state/lyrics.svelte";
  import { startClock, stopClock } from "@state/media-clock.svelte";
  import { toggleLyrics } from "@state/player.svelte";
  import NowPlayingArt from "../player/NowPlayingArt.svelte";
  import KaraokeWords from "./KaraokeWords.svelte";
  import LyricsHelpOverlay from "./LyricsHelpOverlay.svelte";
  import LyricsPasteOverlay from "./LyricsPasteOverlay.svelte";
  import LyricsSearchOverlay from "./LyricsSearchOverlay.svelte";

  let { onclose, fullscreen = false }: { onclose: () => void; fullscreen?: boolean } = $props();

  type Overlay = "help" | "search" | "paste" | null;

  let container = $state<HTMLDivElement>();
  let lineEls = $state<HTMLElement[]>([]);
  let settingsEl = $state<HTMLDivElement>();
  let settingsOpen = $state(false);
  let overlay = $state<Overlay>(null);
  let selected = $state<number[]>([]);
  let autoScroll = $state(readAutoScroll());
  let showRomanization = $state(readRomanization());
  let userScrolling = $state(false);
  let toast = $state("");

  let toastTimer: ReturnType<typeof setTimeout> | null = null;
  let resumeTimer: ReturnType<typeof setTimeout> | null = null;
  let scrollRaf: number | null = null;
  let passiveRaf: number | null = null;
  let initialRaf: number | null = null;
  let passiveLast: number | null = null;
  let lastTrackId: string | null = null;
  let didInitialScroll = false;

  const np = $derived(getCurrentTrack());
  const lines = $derived(getLyricsLines());
  const activeIndex = $derived(getLyricsActiveIndex());
  const synced = $derived(getLyricsSynced());
  const loading = $derived(getLyricsLoading());
  const result = $derived(getLyricsResult());
  const error = $derived(getLyricsError());
  const playing = $derived(getIsTrackPlaying());
  const hasRoman = $derived(lines.some((l) => !!l.roman && l.roman.trim() !== ""));
  const hasContent = $derived(lines.some((l) => l.text.trim() !== "" || isInstrumental(l.text)));

  function readAutoScroll(): boolean {
    if (typeof localStorage === "undefined") return true;
    return localStorage.getItem("vynl.autoscroll") !== "false";
  }

  function setAutoScroll(value: boolean): void {
    autoScroll = value;
    localStorage.setItem("vynl.autoscroll", String(value));
  }

  function readRomanization(): boolean {
    if (typeof localStorage === "undefined") return true;
    return localStorage.getItem("vynl.lyrics.romanization") !== "false";
  }

  function setRomanization(value: boolean): void {
    showRomanization = value;
    localStorage.setItem("vynl.lyrics.romanization", String(value));
  }

  function showToast(message: string): void {
    toast = message;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = ""), 1600);
  }

  function copy(text: string, key = "lyrics.copied", values: Record<string, string | number> = {}): void {
    vynl
      .copyText(text)
      .then(() => showToast(t(key, values)))
      .catch(() => showToast(t("lyrics.copyFailed")));
  }

  function cancelScroll(): void {
    if (scrollRaf !== null) cancelAnimationFrame(scrollRaf);
    scrollRaf = null;
  }

  function cancelPassive(): void {
    if (passiveRaf !== null) cancelAnimationFrame(passiveRaf);
    passiveRaf = null;
    passiveLast = null;
  }

  function prefersReducedMotion(): boolean {
    return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
  }

  function targetScrollTop(index: number): number | null {
    const c = container;
    const el = lineEls[index];
    if (!c || !el) return null;
    const cRect = c.getBoundingClientRect();
    const eRect = el.getBoundingClientRect();
    const current = c.scrollTop;
    const offset = eRect.top - cRect.top + current;
    const topPad = 84;
    const target = offset - topPad - Math.max(0, (c.clientHeight - eRect.height) * 0.18);
    const max = Math.max(0, c.scrollHeight - c.clientHeight);
    return Math.max(0, Math.min(target, max));
  }

  function scrollToActive(immediate = false): void {
    if (!autoScroll || userScrolling || activeIndex < 0) return;
    const c = container;
    const target = targetScrollTop(activeIndex);
    if (!c || target === null) return;
    const delta = target - c.scrollTop;
    if (Math.abs(delta) < 1) return;
    if (immediate || prefersReducedMotion()) {
      cancelScroll();
      c.scrollTop = target;
      return;
    }
    cancelScroll();
    const start = c.scrollTop;
    const distance = target - start;
    const duration = Math.min(560, 180 + Math.abs(distance) * 0.28);
    const t0 = performance.now();
    const step = (now: number) => {
      const p = Math.min(1, (now - t0) / duration);
      const eased = 1 - Math.pow(1 - p, 3);
      c.scrollTop = start + distance * eased;
      if (p < 1) scrollRaf = requestAnimationFrame(step);
      else scrollRaf = null;
    };
    scrollRaf = requestAnimationFrame(step);
  }

  function noteUserScroll(): void {
    userScrolling = true;
    cancelScroll();
    cancelPassive();
    if (resumeTimer) clearTimeout(resumeTimer);
    resumeTimer = setTimeout(() => (userScrolling = false), 4000);
  }

  function jumpToCurrent(): void {
    userScrolling = false;
    if (resumeTimer) clearTimeout(resumeTimer);
    resumeTimer = null;
    if (activeIndex >= 0) scrollToActive(false);
  }

  $effect(() => {
    const id = getCurrentId();
    if (id === lastTrackId) return;
    lastTrackId = id;
    didInitialScroll = false;
    selected = [];
    overlay = null;
    settingsOpen = false;
    cancelScroll();
    loadLyrics(id);
  });

  $effect(() => {
    if (!playing || lines.length === 0) return;
    startClock();
    return () => stopClock();
  });

  $effect(() => {
    result;
    getCurrentId();
    didInitialScroll = false;
  });

  $effect(() => {
    if (activeIndex < 0) return;
    if (!didInitialScroll) {
      didInitialScroll = true;
      initialRaf = requestAnimationFrame(() => {
        initialRaf = null;
        scrollToActive(true);
      });
      return;
    }
    scrollToActive(false);
  });

  $effect(() => {
    return () => {
      if (toastTimer) clearTimeout(toastTimer);
      if (resumeTimer) clearTimeout(resumeTimer);
      if (initialRaf !== null) cancelAnimationFrame(initialRaf);
      cancelScroll();
      cancelPassive();
    };
  });

  $effect(() => {
    const c = container;
    const canScroll =
      !synced && autoScroll && !userScrolling && !overlay && !settingsOpen &&
      selected.length === 0 && !!result && lines.length > 0 && !!c;
    if (!canScroll) {
      cancelPassive();
      return;
    }
    if (prefersReducedMotion()) return;
    const speed = 18;
    const step = (now: number) => {
      const el = container;
      if (!el) { cancelPassive(); return; }
      if (passiveLast === null) passiveLast = now;
      const dt = Math.min(0.05, (now - passiveLast) / 1000);
      passiveLast = now;
      const max = el.scrollHeight - el.clientHeight;
      if (max > 0 && el.scrollTop < max - 1) {
        el.scrollTop = Math.min(max, el.scrollTop + speed * dt);
        passiveRaf = requestAnimationFrame(step);
      } else {
        cancelPassive();
      }
    };
    passiveRaf = requestAnimationFrame(step);
    return () => cancelPassive();
  });

  $effect(() => {
    if (!settingsOpen) return;
    const onDown = (e: PointerEvent) => {
      const target = e.target as HTMLElement;
      if (settingsEl?.contains(target) || target.closest(".lyr-icon")) return;
      settingsOpen = false;
    };
    document.addEventListener("pointerdown", onDown);
    return () => document.removeEventListener("pointerdown", onDown);
  });

  $effect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || e.defaultPrevented) return;
      if (overlay) {
        e.preventDefault();
        overlay = null;
      } else if (settingsOpen) {
        e.preventDefault();
        settingsOpen = false;
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  });

  function onLineClick(e: MouseEvent, i: number): void {
    e.stopPropagation();
    if (e.ctrlKey || e.metaKey) {
      selected = selected.includes(i) ? selected.filter((x) => x !== i) : [...selected, i].sort((a, b) => a - b);
      return;
    }
    const line = lines[i];
    if (synced && line?.time >= 0) {
      seekTo(line.time);
      return;
    }
    selected = selected.length === 1 && selected[0] === i ? [] : [i];
  }

  function onLineContextMenu(e: MouseEvent, i: number): void {
    const text = lines[i]?.text;
    if (!text) return;
    e.preventDefault();
    copy(text);
  }

  function clearSelection(e: PointerEvent): void {
    const target = e.target as HTMLElement;
    if (target.closest(".line") || e.ctrlKey || e.metaKey) return;
    selected = [];
  }

  function copyAll(): void {
    const text = lines.map((l) => l.text.trim()).filter(Boolean).join("\n\n");
    if (text) copy(text, "lyrics.copiedAll", { n: lines.filter((l) => l.text.trim()).length });
  }

  function copySelected(): void {
    const text = selected.map((i) => lines[i]?.text?.trim()).filter((v): v is string => !!v).join("\n\n");
    if (text)
      copy(text, "lyrics.copiedN", {
        n: selected.length,
        unit: selected.length === 1 ? t("lyrics.line") : t("lyrics.lines"),
      });
  }

  function open(next: Exclude<Overlay, null>): void {
    overlay = next;
    settingsOpen = false;
  }
</script>

{#snippet modal(title: string, close: () => void, body: import("svelte").Snippet)}
  <div class="modal" transition:fade={{ duration: 120 }}>
    <button class="backdrop" onclick={close} aria-label={t("common.esc")}></button>
    <div class="modal-card" in:fly={{ y: 8, duration: 180, easing: cubicOut }} out:fly={{ y: 8, duration: 120, easing: cubicIn }}>
      <div class="modal-title display">{title}</div>
      {@render body()}
    </div>
  </div>
{/snippet}

<section class="lyrics-page" class:fullscreen>
  {#if np}
    {#if fullscreen}
      <aside class="art-col">
        <NowPlayingArt
          current={np}
          playing={playing}
          playError={null}
          scanning={false}
          lyricsOpen={true}
          onToggleLyrics={toggleLyrics}
          showLyricsButton={false}
        />
      </aside>
    {/if}

    <div class="main-col">
      {#if !fullscreen}
        <header class="head">
          <div class="track">
            {#if np.cover}
              <img class="cover" use:blobSrc={np.cover} alt="" draggable="false" />
            {:else}
              <div class="cover placeholder">{np.title.charAt(0).toUpperCase()}</div>
            {/if}
            <div class="track-text">
              <div class="title display">{np.title}</div>
              <div class="meta mono">
                <span>{np.artist || t("common.unknownArtist")}</span>
                {#if np.album}<span class="sep">·</span><span class="album">{np.album}</span>{/if}
              </div>
            </div>
          </div>

          <div class="lyr-controls">
            <button class="lyr-icon" onclick={copyAll} aria-label={t("lyrics.copyAllLyrics")} disabled={!hasContent}>
              <Copy size={14} stroke-width={1.5} />
            </button>
            <button class="lyr-icon" class:active={settingsOpen} onclick={() => (settingsOpen = !settingsOpen)} aria-label={t("common.settings")}>
              <Settings2 size={14} stroke-width={1.5} />
            </button>
            <span class="lyr-divider" aria-hidden="true"></span>
            <button class="lyr-icon" onclick={onclose} aria-label={t("lyrics.closeLyrics")}>
              <X size={14} stroke-width={1.5} />
            </button>
          </div>

          {#if settingsOpen}
            <div class="lyr-settings" bind:this={settingsEl} transition:fly={{ y: -6, duration: 150, easing: cubicOut }}>
              <div class="settings-row">
                <span class="settings-label mono">{t("lyrics.autoScroll")}</span>
                <button class="mini-switch" class:on={autoScroll} onclick={() => setAutoScroll(!autoScroll)} aria-label={t("lyrics.toggleAutoScroll")}><span></span></button>
              </div>
              {#if hasRoman}
                <div class="settings-row">
                  <span class="settings-label mono">{t("lyrics.romanization")}</span>
                  <button class="mini-switch" class:on={showRomanization} onclick={() => setRomanization(!showRomanization)} aria-label={t("lyrics.toggleRomanization")}><span></span></button>
                </div>
              {/if}
              <div class="settings-row">
                <button class="btn-ghost settings-action mono" onclick={() => open("search")}>
                  <Search size={13} stroke-width={1.5} />
                  {t("common.search")}
                </button>
              </div>
              <div class="settings-row">
                <button class="btn-ghost settings-action mono" onclick={() => open("paste")}>
                  <ClipboardPaste size={13} stroke-width={1.5} />
                  {t("lyrics.paste")}
                </button>
              </div>
              <div class="settings-row">
                <button class="btn-ghost settings-action mono" onclick={() => open("help")}>
                  <Keyboard size={13} stroke-width={1.5} />
                  {t("lyrics.shortcuts")}
                </button>
              </div>
            </div>
          {/if}
        </header>
      {/if}

      <div
        class="scroll"
        class:synced
        class:paused={!playing}
        bind:this={container}
        role="presentation"
        onpointerdown={clearSelection}
        onwheel={noteUserScroll}
        ontouchmove={noteUserScroll}
      >
        {#if loading && lines.length === 0}
          <div class="empty" transition:fade={{ duration: 120 }} role="status" aria-label={t("lyrics.loadingLyrics")}>
            <div class="skeleton" aria-hidden="true">
              {#each { length: 6 } as _, i (i)}
                <div class="skeleton-line" style:--w={`${70 - (i % 3) * 12}%`}></div>
              {/each}
            </div>
          </div>
        {:else if !result && error}
          <div class="empty" transition:fade={{ duration: 120 }}>
            <div class="empty-title display">{t("lyrics.loadFailed")}</div>
            <button class="retry-btn mono" onclick={() => loadLyrics(getCurrentId(), true)}>{t("common.retry")}</button>
          </div>
        {:else if !result}
          <div class="empty" transition:fade={{ duration: 120 }}>
            <div class="empty-title display">{t("lyrics.noLyricsFound")}</div>
            <button class="retry-btn mono" onclick={() => loadLyrics(getCurrentId(), true)}>{t("common.retry")}</button>
            <div class="empty-actions">
              <button class="retry-btn mono" onclick={() => open("search")}><Search size={13} stroke-width={1.5} />{t("lyrics.searchLyrics")}</button>
              <button class="retry-btn mono" onclick={() => open("paste")}><ClipboardPaste size={13} stroke-width={1.5} />{t("lyrics.paste")}</button>
            </div>
          </div>
        {:else if lines.length === 0}
          <div class="empty mono" transition:fade={{ duration: 120 }}>{t("lyrics.emptyLyricsFile")}</div>
        {:else}
          {#each lines as line, i (i)}
            {@const active = i === activeIndex}
            {@const instrumental = isInstrumental(line.text)}
            {@const hidden = instrumental && !active}
            <button
              type="button"
              class="line"
              class:active
              class:before={synced && activeIndex >= 0 && i < activeIndex}
              class:selected={selected.includes(i)}
              class:bg={!!line.bg}
              class:hidden-line={hidden}
              inert={hidden}
              bind:this={lineEls[i]}
              onclick={(e) => onLineClick(e, i)}
              oncontextmenu={(e) => onLineContextMenu(e, i)}
            >
              <span class="line-inner">
                {#if instrumental}
                  <span class="dots" aria-hidden="true"><span></span><span></span><span></span></span>
                {:else if active && line.words?.length}
                  <KaraokeWords words={line.words} lineEnd={line.end} />
                {:else}
                  <span class="text">{line.text || "\u00A0"}</span>
                {/if}
                {#if showRomanization && line.roman}
                  <span class="roman" class:open={active}><span class="roman-text">{line.roman}</span></span>
                {/if}
              </span>
            </button>
          {/each}
        {/if}
      </div>

      {#if selected.length > 0}
        <div class="selection" transition:fly={{ y: 8, duration: 140, easing: cubicOut }}>
          <span class="selection-count mono">{t("lyrics.nLines", { n: selected.length, unit: selected.length === 1 ? t("lyrics.line") : t("lyrics.lines"), c: selected.reduce((n, i) => n + (lines[i]?.text?.length ?? 0), 0) })}</span>
          <button class="sel-btn" onclick={copySelected} aria-label={t("lyrics.copySelected")}><Copy size={13} stroke-width={1.5} /></button>
          <button class="sel-btn" onclick={() => (selected = [])} aria-label={t("lyrics.clearSelection")}><X size={13} stroke-width={1.5} /></button>
        </div>
      {/if}

      {#if userScrolling && synced && activeIndex >= 0 && autoScroll && selected.length === 0}
        <button class="jump-btn mono" onclick={jumpToCurrent} transition:fly={{ y: 8, duration: 140, easing: cubicOut }}>
          {t("lyrics.jumpToCurrent")}
        </button>
      {/if}
    </div>

    {#if overlay === "help"}{@render modal(t("lyrics.helpTitle"), () => (overlay = null), helpBody)}{/if}
    {#if overlay === "search"}{@render modal(t("lyrics.searchLyrics"), () => (overlay = null), searchBody)}{/if}
    {#if overlay === "paste"}{@render modal(t("lyrics.pasteLyrics"), () => (overlay = null), pasteBody)}{/if}

    {#if toast}
      <div class="toast mono" transition:fade={{ duration: 120 }}>{toast}</div>
    {/if}
  {:else}
    <div class="empty">
      <div class="empty-title display">{t("player.nothingPlaying")}</div>
      <div class="empty-sub mono">{t("lyrics.playToSee")}</div>
    </div>
  {/if}
</section>

{#snippet helpBody()}<LyricsHelpOverlay onClose={() => (overlay = null)} />{/snippet}
{#snippet searchBody()}<LyricsSearchOverlay onClose={() => (overlay = null)} />{/snippet}
{#snippet pasteBody()}<LyricsPasteOverlay onClose={() => (overlay = null)} />{/snippet}

<style>
  :root {
    --ease-out: cubic-bezier(0.22, 1, 0.36, 1);
  }

  .lyrics-page {
    position: relative;
    height: 100%;
    min-height: 0;
    display: flex;
    gap: 20px;
    padding: 20px 0;
  }

  .lyrics-page.fullscreen {
    padding: 24px;
    gap: 24px;
  }

  .art-col {
    width: 360px;
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .art-col :global(.now) {
    padding: 0;
  }
  .art-col :global(.now-art) {
    width: 300px;
    margin: 0;
  }

  .main-col {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .head {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding-bottom: 14px;
  }

  .head::after {
    content: "";
    position: absolute;
    left: 0; right: 0; bottom: 0;
    height: 1px;
    background: linear-gradient(90deg, var(--line-strong), var(--line) 40%, transparent);
  }

  .track {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .cover {
    width: 46px;
    height: 46px;
    border-radius: var(--radius-sm);
    object-fit: cover;
    flex-shrink: 0;
  }
  .cover.placeholder {
    display:flex;
    align-items:center;
    justify-content:center;
    background: var(--placeholder-gradient);
    color: var(--faint);
    font-size: 18px;
  }
  .track-text {
    min-width: 0;
    display:flex;
    flex-direction:column;
    gap:4px;
  }
  .title {
    font-size: 17px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta {
    display:flex;
    gap:6px;
    min-width:0;
    font-size:11px;
    color: var(--dim);
  }
  .meta > span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sep, .album {
    color: var(--faint);
    flex-shrink: 0;
  }
  .album {
    min-width: 0;
    flex-shrink: 1;
  }

  .lyr-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .lyr-divider {
    width: 1px;
    height: 14px;
    background: var(--line);
    margin: 0 2px;
  }

  .lyr-icon {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: 1px solid transparent;
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
    color: var(--text);
    background: var(--hover-surface);
    border-color: var(--hover-border);
  }
  .lyr-icon.active {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
    background: color-mix(in srgb, var(--accent) 20%, transparent);
  }
  .lyr-icon:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .lyr-icon:disabled:hover {
    color: var(--faint);
    background: none;
    border-color: transparent;
  }

  .lyr-settings {
    position: absolute;
    top: calc(100% + 10px);
    right: 0;
    z-index: 30;
    min-width: 190px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    box-shadow: var(--shadow-md);
  }
  .settings-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 7px 8px;
    border-radius: var(--radius-sm);
    transition: background 0.12s;
  }
  .settings-row:hover {
    background: var(--hover-surface);
  }
  .settings-label {
    font-size: 10.5px;
    letter-spacing: 0.05em;
    color: var(--dim);
  }
  .mini-switch {
    width: 28px;
    height: 15px;
    border-radius: var(--radius-sm);
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
    border-radius: var(--radius-xs);
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
  .settings-action {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 9px;
    text-align: left;
  }
  .settings-action :global(svg) {
    color: var(--faint);
    flex-shrink: 0;
    transition: color 0.15s;
  }
  .btn-ghost.settings-action:hover :global(svg) {
    color: var(--accent);
  }

  .scroll {
    --fade-top: 56px;
    --fade-bottom: 64px;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: none;
    padding: calc(var(--fade-top) + 36px) 20px 12vh;
    mask-image: linear-gradient(to bottom, transparent 0, transparent 12px, black var(--fade-top), black calc(100% - var(--fade-bottom)), transparent 100%);
    -webkit-mask-image: linear-gradient(to bottom, transparent 0, transparent 12px, black var(--fade-top), black calc(100% - var(--fade-bottom)), transparent 100%);
  }
  .scroll::-webkit-scrollbar {
    display: none;
  }
  .scroll.synced {
    padding-bottom: 18vh;
  }

  .empty {
    margin: auto;
    text-align:center;
    color: var(--faint);
    font-size: 11.5px;
  }
  .empty-title {
    font-size: 20px;
    color: var(--dim);
    margin-bottom: 6px;
  }
  .empty-sub {
    font-size: 11px;
  }
  .retry-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    justify-content: center;
    margin-top: 12px;
    padding: 7px 14px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line-strong);
    background: var(--bg-raise);
    color: var(--dim);
    font-size: 11px;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s, background 0.15s;
  }
  .retry-btn:hover {
    color: var(--accent);
    border-color: var(--hover-border);
    background: var(--hover-surface);
  }
  .retry-btn :global(svg) {
    color: var(--faint);
    transition: color 0.15s;
  }
  .retry-btn:hover :global(svg) {
    color: var(--accent);
  }
  .empty-actions {
    display: flex;
    gap: 8px;
    justify-content: center;
  }
  .skeleton {
    width: 100%;
    max-width: 560px;
    margin: 8px auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .skeleton-line {
    height: 18px; width: var(--w, 70%); margin: 0 auto; border-radius: 6px;
    background: var(--bg-raise); animation: skel-pulse 1.2s ease-in-out infinite;
  }
  @keyframes skel-pulse {
    0%,
    100% {
      opacity: 0.45;
    }
    50% {
      opacity: 0.85;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .skeleton-line {
      animation: none;
    }
  }

  .line {
    width: 100%;
    max-width: 760px;
    margin: 0 auto 18px;
    padding: 0 16px;
    border: none;
    background: none;
    color: var(--faint);
    text-align: center;
    font-family: var(--font-lyrics);
    font-size: 26px;
    line-height: 1.9;
    font-weight: 500;
    cursor: pointer;
    display: grid;
    grid-template-rows: 1fr;
    transform-origin: center;
    transition: grid-template-rows .32s cubic-bezier(.22,1,.36,1), margin .32s cubic-bezier(.22,1,.36,1), transform .45s cubic-bezier(.22,1,.36,1), color .35s, opacity .3s;
  }
  .line:last-child {
    margin-bottom: 0;
  }
  .line-inner {
    display:block;
    min-height:0;
    overflow:hidden;
    padding: 10px 0;
  }
  .line.active {
    color: var(--text);
    font-weight: 600;
    transform: scale(1.12);
  }
  .line.before {
    color: color-mix(in srgb, var(--dim) 62%, var(--bg));
    transform: scale(0.985);
  }
  .line.selected {
    color: var(--accent);
  }
  .line.bg {
    font-size: 20px;
  }
  .line.hidden-line {
    grid-template-rows: 0fr;
    margin-bottom: 0;
    opacity: 0;
    pointer-events: none;
  }

  .roman {
    display:grid;
    grid-template-rows: 0fr;
    opacity: 0;
    margin-top: 0;
    transition: grid-template-rows .3s, opacity .3s, margin .3s;
  }
  .roman.open {
    grid-template-rows: 1fr;
    opacity: 1;
    margin-top: 6px;
  }
  .roman-text {
    display:block;
    min-height:0;
    overflow:hidden;
    font-size:14px;
    line-height:1.4;
    color: color-mix(in srgb, var(--dim) 70%, var(--bg));
  }

  .dots {
    display:inline-flex;
    gap:10px;
    align-items:center;
  }
  .dots span {
    width:10px;
    height:10px;
    border-radius:50%;
    background: currentColor;
    opacity:.45;
    animation: dot 1.4s cubic-bezier(.45,0,.55,1) infinite;
  }
  .dots span:nth-child(2) {
    animation-delay: .2s;
  }
  .dots span:nth-child(3) {
    animation-delay: .4s;
  }
  .scroll.paused .dots span, .line.hidden-line .dots span {
    animation-play-state: paused;
  }
  @keyframes dot {
    0%,
    100% {
      transform: translateY(4px) scale(0.85);
      opacity: 0.35;
    }
    50% {
      transform: translateY(-4px) scale(1.05);
      opacity: 1;
    }
  }

  .selection {
    position: absolute; left: 50%; bottom: 12px; transform: translateX(-50%); z-index: 30;
    display:flex; align-items:center; gap:4px; padding:5px 6px 5px 14px;
    background: var(--surface); border: 1px solid var(--line-strong); border-radius: var(--radius-sm);
  }
  .jump-btn {
    position: absolute; left: 50%; bottom: 12px; transform: translateX(-50%); z-index: 30;
    padding: 8px 14px; font-size: 10.5px; letter-spacing: .05em;
    color: var(--dim); background: var(--surface); border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm); cursor: pointer;
  }
  .jump-btn:hover {
    color: var(--accent);
    border-color: var(--hover-border);
    background: var(--hover-surface);
  }
  .selection-count {
    font-size: 10px;
    color: var(--dim);
    letter-spacing: 0.05em;
    padding-right: 6px;
  }
  .sel-btn {
    border: none;
    background: none;
    color: var(--faint);
    padding: 5px;
    cursor: pointer;
    transition: color 0.15s;
  }
  .sel-btn:hover {
    color: var(--accent);
  }

  .modal {
    position:absolute;
    inset:0;
    z-index:40;
    display:flex;
    align-items:center;
    justify-content:center;
  }
  .backdrop {
    position:fixed;
    inset:0;
    border:none;
    background: rgba(0,0,0,.3);
    backdrop-filter: blur(12px);
    cursor:pointer;
  }
  .modal-card {
    position:relative; min-width:360px; max-width:460px; max-height:80%; overflow:hidden;
    display:flex; flex-direction:column; gap:14px; padding:24px 28px;
    background: var(--surface); border:1px solid var(--line-strong); border-radius: var(--radius-sm);
  }
  .modal-title {
    font-size: 17px;
  }

  .toast {
    position:absolute; left:50%; bottom:56px; transform:translateX(-50%); z-index:50;
    padding:8px 18px; font-size:10.5px; letter-spacing:.05em; color:var(--text);
    background: var(--bg-raise); border:1px solid var(--line-strong); border-radius: var(--radius-sm);
  }

  @media (max-width: 900px) {
    .lyrics-page.fullscreen {
      flex-direction: column;
    }
    .art-col {
      width: 100%;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .line, .lyr-icon, .dots span, .mini-switch span {
      transition: none !important;
      animation: none !important;
    }
  }
</style>
