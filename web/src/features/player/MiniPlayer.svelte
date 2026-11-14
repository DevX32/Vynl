<script lang="ts">
  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    Play,
    Pause,
    SkipBack,
    SkipForward,
    Maximize2,
    Volume2,
    VolumeOff,
    Music2,
  } from "lucide-svelte";
  import { getLibrary, refreshLibrary } from "@state/library.svelte";
  import { vynl } from "@lib/vynl";
  import { applyAccentColor } from "@lib/color";
  import {
    MINI_STATE_EVENT,
    requestMiniState,
    sendMiniCommand,
    sendMiniClosed,
  } from "@lib/mini-sync";
  import { blobSrc, fmtTime } from "../../lib/format";
  import { pointerRatio } from "../../lib/pointer";
  import { t } from "@lib/i18n";
  import type { MiniSyncState } from "@lib/types";

  const win = getCurrentWindow();

  const VINYL_DEG_PER_SEC = (360 / 60) * (33 + 1 / 3);
  const SPEED_EPSILON = 0.5;
  const EASE = 0.03;

  let track = $state<MiniSyncState["track"]>(null);
  let playing = $state(false);
  let time = $state(0);
  let previewTime = $state(0);
  let duration = $state(0);
  let vinylDeg = $state(0);
  let vinylSpeed = 0;
  let raf = 0;
  let lastTs = 0;
  let seeking = $state(false);
  let volDragging = $state(false);

  const VOLUME_KEY = "vynl.volume";
  const MUTED_KEY = "vynl.muted";

  function loadVolume(): number {
    if (typeof localStorage === "undefined") return 1;
    const raw = Number(localStorage.getItem(VOLUME_KEY) ?? "1");
    return Number.isFinite(raw) ? Math.min(1, Math.max(0, raw)) : 0.5;
  }

  function loadMuted(): boolean {
    if (typeof localStorage === "undefined") return false;
    return localStorage.getItem(MUTED_KEY) === "1";
  }

  let volume = $state(loadVolume());
  let muted = $state(loadMuted());

  function syncVolumeFromStorage(): void {
    volume = loadVolume();
    muted = loadMuted();
  }

  function setVolume(ratio: number): void {
    const next = Math.min(1, Math.max(0, ratio));
    volume = next;
    muted = next === 0 ? loadMuted() : false;
    localStorage.setItem(VOLUME_KEY, String(next));
    if (next > 0) localStorage.setItem(MUTED_KEY, "0");
    sendMiniCommand({ type: "volume", value: next });
  }

  function toggleMute(): void {
    muted = !muted;
    localStorage.setItem(MUTED_KEY, muted ? "1" : "0");
    sendMiniCommand({ type: "toggleMute" });
  }

  function clampRatio(ratio: number): number {
    return Math.min(1, Math.max(0, ratio));
  }

  function previewSeekAt(ratio: number): void {
    if (duration <= 0) return;
    previewTime = clampRatio(ratio) * duration;
  }

  function commitSeek(value: number): void {
    if (!Number.isFinite(value)) return;
    time = Math.min(duration, Math.max(0, value));
    sendMiniCommand({ type: "seek", value: time });
  }

  function onSeekPointerDown(e: PointerEvent): void {
    const el = e.currentTarget as HTMLDivElement;
    seeking = true;
    const target = pointerRatio(el, e.clientX);
    previewSeekAt(target);
    const move = (ev: PointerEvent): void => {
      previewSeekAt(pointerRatio(el, ev.clientX));
    };
    const up = (): void => {
      const value = previewTime;
      seeking = false;
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", up);
      commitSeek(value);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", up);
  }

  function onVolumePointerDown(e: PointerEvent): void {
    const el = e.currentTarget as HTMLDivElement;
    volDragging = true;
    const move = (ev: PointerEvent): void => {
      setVolume(pointerRatio(el, ev.clientX));
    };
    const up = (): void => {
      volDragging = false;
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    setVolume(pointerRatio(el, e.clientX));
  }

  function loop(target: number, ts: number): void {
    const dt = lastTs ? Math.min((ts - lastTs) / 1000, 0.25) : 0;
    lastTs = ts;
    if (dt > 0) {
      const blend = 1 - Math.pow(1 - EASE, dt * 60);
      vinylSpeed += (target - vinylSpeed) * blend;
      vinylDeg = (vinylDeg + vinylSpeed * dt) % 360;
    }
    if (target === 0 && Math.abs(vinylSpeed) < SPEED_EPSILON) {
      vinylSpeed = 0;
      lastTs = 0;
      raf = 0;
      return;
    }
    raf = requestAnimationFrame((nextTs) => loop(target, nextTs));
  }

  $effect(() => {
    const target = playing ? VINYL_DEG_PER_SEC : 0;
    lastTs = 0;
    raf = requestAnimationFrame((ts) => loop(target, ts));
    return () => {
      cancelAnimationFrame(raf);
      raf = 0;
      lastTs = 0;
    };
  });

  onMount(() => {
    void (async () => {
      try {
        const settings = await vynl.getSettings();
        applyAccentColor(settings.accentColor, false);
      } catch {

      }
    })();
    requestMiniState();

    const offTick = vynl.onPlaybackTick((tick) => {
      time = tick.position;
      if (tick.duration && tick.duration > 0) duration = tick.duration;
      playing = tick.playing && !tick.finished;
    });

    let disposed = false;
    let offSync: UnlistenFn | null = null;
    let offClose: UnlistenFn | null = null;
    void listen<MiniSyncState>(MINI_STATE_EVENT, (e) => {
      const nextState = e.payload;
      if (!nextState) return;
      track = nextState.track;
      playing = nextState.playing;
      if (nextState.track) {
        duration = nextState.track.duration;
      } else {
        duration = 0;
        time = 0;
      }
    }).then((fn) => {
      if (disposed) fn();
      else offSync = fn;
    });

    void win
      .onCloseRequested((event) => {
        event.preventDefault();
        sendMiniClosed();
        void win.hide();
      })
      .then((fn) => {
        if (disposed) fn();
        else offClose = fn;
      });

    const onStorage = (): void => syncVolumeFromStorage();
    window.addEventListener("storage", onStorage);

    const onKey = (e: KeyboardEvent): void => {
      if (e.repeat) return;
      if (e.code === "Space") {
        e.preventDefault();
        sendMiniCommand({ type: "toggle" });
      } else if (e.code === "Escape") {
        expand();
      } else if (duration > 0 && e.code === "ArrowRight") {
        e.preventDefault();
        time = Math.min(duration, time + 5);
        sendMiniCommand({ type: "seek", value: time });
      } else if (duration > 0 && e.code === "ArrowLeft") {
        e.preventDefault();
        time = Math.max(0, time - 5);
        sendMiniCommand({ type: "seek", value: time });
      }
    };
    window.addEventListener("keydown", onKey);

    void (async () => {
      try {
        await refreshLibrary();
        const saved = await vynl.loadNowPlaying();
        if (!saved) return;
        const tracks = getLibrary();
        const hit =
          tracks.find((item) => item.id === saved.id) ??
          tracks.find((item) => item.path === saved.path);
        if (hit) {
          track = {
            id: hit.id,
            path: hit.path,
            title: hit.title,
            artist: hit.artist,
            album: hit.album,
            cover: hit.cover,
            duration: hit.duration,
          };
          if (hit.duration > 0) duration = hit.duration;
        }
        time = saved.time ?? 0;
      } catch (e) {
        console.warn("mini restore failed:", e);
      }
    })();

    return () => {
      disposed = true;
      offTick();
      offSync?.();
      offClose?.();
      window.removeEventListener("storage", onStorage);
      window.removeEventListener("keydown", onKey);
    };
  });

  function expand(): void {
    sendMiniCommand({ type: "expand" });
    sendMiniClosed();
    void win.hide();
  }



  function onHeadDoubleClick(e: MouseEvent): void {
    if (e.target === e.currentTarget) expand();
  }

  const displayTime = $derived(seeking ? previewTime : time);
  const seekRatio = $derived(duration > 0 ? Math.min(1, displayTime / duration) : 0);
  const volRatio = $derived(volume);
</script>

<svelte:head>
  <style>
    html,
    body {
      background: transparent !important;
    }
  </style>
</svelte:head>

<div class="mini">
  <div class="ambient" aria-hidden="true"></div>
  {#if track?.cover}
    <div class="backdrop" aria-hidden="true">
      <img class="backdrop-img" use:blobSrc={track.cover} alt="" />
    </div>
  {/if}
  <div class="scrim" aria-hidden="true"></div>
  <div class="grain" aria-hidden="true"></div>

      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="mini-head"
    data-tauri-drag-region
    ondblclick={onHeadDoubleClick}
  >
    <span class="mini-brand display" data-tauri-drag-region aria-hidden="true"
      >VYNL</span
    >
    <div class="mini-head-btns">
      <button
        class="mini-btn"
        onclick={expand}
        aria-label={t("player.expandPlayer")}
      >
        <Maximize2 size={12} stroke-width={1.5} />
      </button>
    </div>
  </div>

  <div class="mini-body">
    <div class="stage" data-tauri-drag-region="deep">
      <div class="disc" class:playing class:empty={!track}>
        <div class="vinyl-slot">
          <div
            class="vinyl"
            style:transform={`rotate(${vinylDeg}deg)`}
            aria-hidden="true"
          ></div>
        </div>
        <div class="sleeve">
          {#if track?.cover}
            <img class="cover" use:blobSrc={track.cover} alt="" draggable="false" />
          {:else}
            <div class="cover placeholder">
              {#if track}
                <span class="ph-initial display"
                  >{track.title.charAt(0).toUpperCase()}</span
                >
              {:else}
                <span class="ph">
                  <Music2 size={17} stroke-width={1.1} />
                  <span class="ph-mark display">VYNL</span>
                </span>
              {/if}
            </div>
          {/if}
        </div>
      </div>
    </div>

    <div class="info">
      <span class="title display" class:muted={!track}
        >{track?.title ?? t("player.nothingPlaying")}</span
      >
      {#if track}
        <span class="artist">{track.artist}</span>
      {/if}
    </div>

    <div class="seek-row">
      <span class="time mono">{fmtTime(displayTime)}</span>

      <div
        class="slider seek"
        class:disabled={duration <= 0}
        class:dragging={seeking}
        role="slider"
        aria-label={t("player.seek")}
        aria-valuemin={0}
        aria-valuemax={duration}
        aria-valuenow={Math.round(displayTime)}
        tabindex="0"
        onpointerdown={onSeekPointerDown}
      >
        <div class="rail">
          <div class="fill" style:width={`${seekRatio * 100}%`}></div>
        </div>
        <div class="knob" style:left={`${seekRatio * 100}%`}></div>
      </div>
      <span class="time mono end">{fmtTime(duration)}</span>
    </div>

    <div class="transport">
      <button
        class="icon-btn"
        onclick={() => sendMiniCommand({ type: "prev" })}
        aria-label={t("player.previous")}
        disabled={!track}
      >
        <SkipBack size={14} fill="currentColor" stroke-width={0} />
      </button>
      <button
        class="play-btn"
        onclick={() => sendMiniCommand({ type: "toggle" })}
        aria-label={playing ? t("player.pause") : t("player.play")}
        disabled={!track}
      >
        <span class="p-icon" class:show={playing}>
          <Pause size={14} fill="currentColor" stroke-width={0} />
        </span>
        <span class="p-icon" class:show={!playing}>
          <Play size={14} fill="currentColor" stroke-width={0} />
        </span>
      </button>
      <button
        class="icon-btn"
        onclick={() => sendMiniCommand({ type: "next" })}
        aria-label={t("player.next")}
        disabled={!track}
      >
        <SkipForward size={14} fill="currentColor" stroke-width={0} />
      </button>
    </div>

    <div class="vol-row">
      <button
        class="vol-btn"
        onclick={toggleMute}
        aria-label={muted ? t("player.unmute") : t("player.mute")}
      >
        {#if muted || volume === 0}
          <VolumeOff size={13} stroke-width={1.5} />
        {:else}
          <Volume2 size={13} stroke-width={1.5} />
        {/if}
      </button>

      <div
        class="slider vol"
        class:dragging={volDragging}
        role="slider"
        aria-label={t("player.volume")}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(volRatio * 100)}
        tabindex="0"
        onpointerdown={onVolumePointerDown}
      >
        <div class="rail">
          <div class="fill" style:width={`${volRatio * 100}%`}></div>
        </div>
        <div class="knob" style:left={`${volRatio * 100}%`}></div>
      </div>
    </div>
  </div>
</div>

<style>
  .mini {
    position: relative;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--text);
    overflow: hidden;
    user-select: none;
    isolation: isolate;
    border-radius: var(--radius-sm);
  }

  .mini::after {
    content: "";
    position: absolute;
    inset: 0;
    z-index: 20;
    border: 1px solid rgba(255, 255, 255, 0.055);
    border-radius: var(--radius-sm);
    pointer-events: none;
  }

  .ambient {
    position: absolute;
    inset: 0;
    z-index: 0;
    pointer-events: none;
    background: var(--bg);
  }

  .backdrop {
    position: absolute;
    inset: -25%;
    z-index: 0;
    overflow: hidden;
    pointer-events: none;
    opacity: 0.6;
  }

  .backdrop-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: blur(44px) saturate(1.55) brightness(0.62);
    transform: scale(1.15);
  }

  .scrim {
    position: absolute;
    inset: 0;
    z-index: 1;
    pointer-events: none;
    background:
      radial-gradient(
        ellipse 85% 55% at 50% 32%,
        transparent 0%,
        rgba(14, 14, 16, 0.28) 68%,
        rgba(14, 14, 16, 0.72) 100%
      ),
      linear-gradient(
        180deg,
        rgba(14, 14, 16, 0.2) 0%,
        transparent 28%,
        rgba(14, 14, 16, 0.45) 72%,
        rgba(14, 14, 16, 0.92) 100%
      );
  }

  .grain {
    position: absolute;
    inset: 0;
    z-index: 2;
    pointer-events: none;
    opacity: 0.045;
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
    background-repeat: repeat;
    background-size: 160px;
  }

  .mini-head {
    position: relative;
    z-index: 4;
    flex-shrink: 0;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 6px 0 14px;
    -webkit-app-region: drag;
  }

  .mini-brand {
    font-size: 9px;
    letter-spacing: 0.34em;
    color: color-mix(in srgb, var(--text) 52%, transparent);
  }

  .mini-head-btns {
    display: flex;
    gap: 1px;
    -webkit-app-region: no-drag;
  }

  .mini-btn {
    position: relative;
    width: 26px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: none;
    color: var(--faint);
    cursor: pointer;
    transition:
      color 0.1s,
      transform 0.1s;
  }

  .mini-btn::before {
    content: "";
    position: absolute;
    inset: 3px;
    border-radius: var(--radius-sm);
    background: transparent;
    transform: rotate(45deg);
    transition: background 0.1s;
  }

  .mini-btn :global(svg) {
    position: relative;
    z-index: 1;
  }

  .mini-btn:hover {
    color: var(--text);
  }

  .mini-btn:hover::before {
    background: rgba(255, 255, 255, 0.06);
  }

  .mini-btn:active {
    transform: scale(0.9);
  }

  .mini-btn:active::before {
    background: rgba(255, 255, 255, 0.04);
  }

  .mini-body {
    position: relative;
    z-index: 3;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 8px 18px 14px;
  }

  .stage {
    flex: 1;
    min-height: 0;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px 0;
  }

  .disc {
    position: relative;
    width: 182px;
    aspect-ratio: 1 / 1;
    isolation: isolate;
  }

  .vinyl-slot {
    position: absolute;
    inset: 0;
    z-index: 1;
    transform: translateX(3.5%);
    transition:
      transform 0.55s cubic-bezier(0.22, 1, 0.36, 1),
      filter 0.5s ease;
    will-change: transform;
  }

  .disc.playing .vinyl-slot {
    transform: translateX(14.5%);
  }

  .disc.empty .vinyl-slot {
    transform: translateX(1.5%);
    filter: saturate(0.35) brightness(0.72);
  }

  .vinyl {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: var(--vinyl);
    overflow: hidden;
    will-change: transform;
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.06),
      inset 0 0 0 4px rgba(0, 0, 0, 0.5),
      inset 0 0 20px 3px rgba(0, 0, 0, 0.6);
  }

  .vinyl::before {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background:
      repeating-radial-gradient(
        circle at 50% 50%,
        rgba(255, 255, 255, 0.05) 0 0.5%,
        transparent 0.5% 2.9%
      ),
      repeating-radial-gradient(
        circle at 50% 50%,
        rgba(255, 255, 255, 0.028) 0 0.34%,
        transparent 0.34% 1.63%
      );
    mask-image: radial-gradient(
      circle at 50% 50%,
      transparent 0 12%,
      black 13.5%
    );
    -webkit-mask-image: radial-gradient(
      circle at 50% 50%,
      transparent 0 12%,
      black 13.5%
    );
  }

  .vinyl-slot::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background:
      conic-gradient(
        from 200deg at 50% 50%,
        transparent 0deg,
        rgba(255, 255, 255, 0.07) 24deg,
        transparent 58deg,
        transparent 188deg,
        rgba(255, 255, 255, 0.05) 214deg,
        transparent 250deg
      ),
      linear-gradient(
        150deg,
        rgba(255, 255, 255, 0.1) 0%,
        transparent 32%
      );
    mix-blend-mode: screen;
    pointer-events: none;
  }

  .sleeve {
    position: absolute;
    z-index: 2;
    left: 50%;
    top: 50%;
    width: 72%;
    height: 72%;
    transform: translate(-50%, -50%) translateX(-10px);
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg-raise);
    box-shadow: 0 0 0 1px var(--line);
    transition: transform 0.55s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .disc:not(.playing) .sleeve {
    transform: translate(-50%, -50%) translateX(-22px);
  }

  .disc.playing .sleeve {
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 65%, transparent);
  }

  .cover {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    background: var(--bg-raise);
  }

  .cover.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background:
      linear-gradient(
        155deg,
        rgba(255, 255, 255, 0.05) 0%,
        transparent 55%
      ),
      var(--placeholder-gradient);
    color: var(--faint);
  }

  .ph {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    opacity: 0.85;
  }

  .ph-mark {
    font-size: 8.5px;
    letter-spacing: 0.34em;
    text-indent: 0.34em;
  }

  .ph-initial {
    font-size: 42px;
    line-height: 1;
    opacity: 0.7;
  }

  .info {
    flex-shrink: 0;
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    min-width: 0;
  }

  .title {
    max-width: 100%;
    font-size: 16px;
    line-height: 1.25;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .title.muted {
    color: var(--dim);
  }

  .artist {
    max-width: 100%;
    margin-top: 2px;
    font-family: var(--font-display);
    font-size: 10px;
    letter-spacing: 0.02em;
    color: var(--faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .seek-row {
    flex-shrink: 0;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
  }

  .time {
    min-width: 28px;
    font-size: 10.5px;
    line-height: 1;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }

  .time.end {
    text-align: center;
  }

  .slider {
    position: relative;
    flex: 1;
    height: 16px;
    display: flex;
    align-items: center;
    border-radius: 2px;
    cursor: pointer;
    touch-action: none;
  }

  .slider.disabled {
    cursor: default;
    pointer-events: none;
  }

  .rail {
    position: relative;
    width: 100%;
    height: 3px;
    border-radius: var(--radius-sm);
    background: var(--line-strong);
    overflow: hidden;
  }

  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: var(--radius-sm);
    background: var(--accent);
  }

  .knob {
    position: absolute;
    top: 50%;
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 20%, transparent);
    transform: translate(-50%, -50%) rotate(45deg);
    opacity: 0;
    transition:
      opacity 0.18s ease-out,
      box-shadow 0.18s ease-out;
    pointer-events: none;
  }

  .slider:hover .knob,
  .slider.dragging .knob {
    opacity: 1;
  }

  .transport {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 18px;
    margin-top: 14px;
  }

  .transport .icon-btn {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line);
    color: var(--faint);
    transform: rotate(45deg);
    transition:
      transform 0.15s,
      color 0.15s,
      background 0.15s,
      border-color 0.15s,
      box-shadow 0.15s;
  }

  .transport .icon-btn :global(svg) {
    transform: rotate(-45deg);
  }

  .transport .icon-btn:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--line-strong);
    box-shadow: var(--shadow-sm);
  }

  .transport .icon-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .transport .play-btn {
    position: relative;
    width: 40px;
    height: 40px;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--accent-text);
    display: flex;
    align-items: center;
    justify-content: center;
    transform: rotate(45deg);
    transition:
      background 0.15s,
      transform 0.1s,
      box-shadow 0.15s;
  }

  .transport .play-btn .p-icon {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 0.18s ease-out;
  }

  .transport .play-btn .p-icon.show {
    opacity: 1;
  }

  .transport .play-btn :global(svg) {
    transform: rotate(-45deg);
  }

  .transport .play-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 82%, #ffffff);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .transport .play-btn:active:not(:disabled) {
    transform: rotate(45deg) scale(0.92);
  }

  .transport .play-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .vol-row {
    flex-shrink: 0;
    width: auto;
    display: flex;
    align-items: center;
    gap: 5px;
    margin-top: 14px;
  }

  .vol-btn {
    width: 22px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: none;
    background: none;
    color: var(--faint);
    cursor: pointer;
    transition: color 0.15s;
  }

  .vol-btn:hover {
    color: var(--text);
  }

  .slider.vol {
    flex: 0 0 120px;
  }
</style>
