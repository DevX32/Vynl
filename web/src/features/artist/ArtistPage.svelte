<script lang="ts">
  import { fade } from "svelte/transition";
  import {
    X,
    Music,
    MapPin,
    Calendar,
    User,
    Tag,
    ChevronDown,
  } from "lucide-svelte";
  import { t } from "@lib/i18n";
  import { loadArtistInfo, peekArtistInfo } from "@lib/artist-cache";
  import type { ArtistInfo } from "@lib/types";

  let {
    artist,
    onBack,
  }: {
    artist: string;
    onBack: () => void;
  } = $props();

  let info = $state<ArtistInfo | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let bioExpanded = $state(false);

  $effect(() => {
    const name = artist;
    let cancelled = false;
    const peek = peekArtistInfo(name);
    if (peek) {
      info = peek;
      loading = false;
      error = null;
      return;
    }
    loading = true;
    error = null;
    info = null;
    loadArtistInfo(name)
      .then((data) => {
        if (!cancelled && name === artist) {
          info = data;
          loading = false;
        }
      })
      .catch((e) => {
        if (!cancelled && name === artist) {
          error = String(e);
          loading = false;
        }
      });
    return () => {
      cancelled = true;
    };
  });

  const countryName = $derived.by(() => {
    const code = info?.country;
    if (!code) return null;
    try {
      return new Intl.DisplayNames(["en"], { type: "region" }).of(code) ?? code;
    } catch {
      return code;
    }
  });

  const activeYears = $derived.by(() => {
    const ls = info?.lifeSpan;
    if (!ls?.begin) return null;
    const begin = ls.begin.slice(0, 4);
    if (ls.ended && ls.end) {
      return `${begin}–${ls.end.slice(0, 4)}`;
    }
    return `${begin}–present`;
  });

  const artistType = $derived.by(() => {
    const tp = info?.artistType;
    if (!tp) return null;
    return tp.charAt(0).toUpperCase() + tp.slice(1);
  });

  const metaLine = $derived.by(() => {
    const parts: string[] = [];
    if (artistType) parts.push(artistType);
    if (countryName) {
      parts.push(info?.beginArea ? `${countryName} · ${info.beginArea}` : countryName);
    } else if (info?.beginArea) {
      parts.push(info.beginArea);
    }
    if (activeYears) parts.push(activeYears);
    return parts.join(" · ");
  });

  const hasBio = $derived(info?.bio && info.bio.trim().length > 0);
  const bioText = $derived(info?.bio?.trim() ?? "");
  const displayBio = $derived(bioText);

  let viewEl: HTMLElement | undefined = $state();
  let bioEl: HTMLElement | undefined = $state();
  let bioMaxHeight = $state<number | null>(null);
  let bioCollapsedHeight = $state<number | null>(null);
  let bioOverflows = $state(false);

  const BIO_LINES = 5;
  const BIO_DURATION_MS = 320;

  function measureBio(): void {
    const el = bioEl;
    if (!el) return;

    const natural = el.scrollHeight;
    const lineHeight = parseFloat(getComputedStyle(el.querySelector(".bio")!).lineHeight);
    const line = Number.isFinite(lineHeight) && lineHeight > 0 ? lineHeight : 23.8;
    const collapsed = Math.min(line * BIO_LINES, natural);

    bioOverflows = natural > collapsed + 1;
    bioCollapsedHeight = collapsed;

    if (!bioExpanded) {
      bioMaxHeight = collapsed;
    }
  }

  function toggleBio(): void {
    const el = bioEl;
    const next = !bioExpanded;

    if (el && !next) {
      const viewRect = viewEl?.getBoundingClientRect();
      const bioRect = el.getBoundingClientRect();
      if (viewRect && bioRect.top < viewRect.top + 24) {
        viewEl?.scrollTo({
          top: (viewEl?.scrollTop ?? 0) + (bioRect.top - viewRect.top) - 24,
          behavior: "smooth",
        });
      }
    }

    bioExpanded = next;

    const target = next ? el?.scrollHeight : bioCollapsedHeight;
    if (target != null) {
      bioMaxHeight = target;
    }

    if (next && el) {
      const natural = el.scrollHeight;
      window.setTimeout(() => {
        if (bioExpanded && bioEl) bioMaxHeight = natural;
      }, BIO_DURATION_MS + 40);
    }
  }

  $effect(() => {
    const el = bioEl;
    const text = bioText;
    if (!el || !text) return;

    measureBio();
    const frame = requestAnimationFrame(measureBio);

    const observer = new ResizeObserver(() => {
      if (!bioExpanded) measureBio();
    });
    observer.observe(el);

    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  });
</script>

<section class="artist-page">
  <div class="view" bind:this={viewEl} in:fade={{ duration: 150 }} out:fade={{ duration: 100 }}>
    <button class="close" onclick={onBack} aria-label={t("common.close")}>
      <X size={16} stroke-width={1.5} />
    </button>

    <div class="header">
      <div class="name-block">
        <h1 class="name">{artist}</h1>
        {#if !loading && metaLine}
          <p class="meta">
            {#if artistType}
              <span class="meta-item">
                <User size={12} stroke-width={1.5} aria-hidden="true" />
                {artistType}
              </span>
            {/if}
            {#if countryName}
              <span class="meta-item">
                <MapPin size={12} stroke-width={1.5} aria-hidden="true" />
                {countryName}
                {#if info?.beginArea} · {info.beginArea}{/if}
              </span>
            {:else if info?.beginArea}
              <span class="meta-item">
                <MapPin size={12} stroke-width={1.5} aria-hidden="true" />
                {info.beginArea}
              </span>
            {/if}
            {#if activeYears}
              <span class="meta-item">
                <Calendar size={12} stroke-width={1.5} aria-hidden="true" />
                {activeYears}
              </span>
            {/if}
          </p>
        {/if}
      </div>
    </div>

    {#if loading}
      <div class="loading-state">
        <div class="spinner" aria-hidden="true"></div>
        <p>{t("artist.loading")}</p>
      </div>
    {:else if error}
      <div class="error-state">
        <p class="err">{t("artist.loadError")}</p>
      </div>
    {:else if info}
      {#if hasBio}
        <div class="section">
          <h2 class="heading">{t("artist.bio")}</h2>
          <div
            class="bio-clip"
            bind:this={bioEl}
            style:max-height={bioMaxHeight == null ? undefined : `${bioMaxHeight}px`}
          >
            <p class="bio">{displayBio}</p>
          </div>
          {#if bioOverflows}
            <button class="more" onclick={toggleBio} aria-expanded={bioExpanded}>
              <span class="more-label">
                {bioExpanded ? t("artist.showLess") : t("artist.readMore")}
              </span>
              <ChevronDown
                size={12}
                stroke-width={1.5}
                class="more-icon"
                aria-hidden="true"
              />
            </button>
          {/if}
        </div>
      {/if}

      {#if info.genres.length > 0}
        <div class="section">
          <h2 class="heading">
            <Music size={10} stroke-width={1.5} aria-hidden="true" />
            <span>{t("artist.genres")}</span>
          </h2>
          <div class="chips">
            {#each info.genres as genre}
              <span class="chip">{genre}</span>
            {/each}
          </div>
        </div>
      {/if}

      {#if info.tags.length > 0}
        <div class="section">
          <h2 class="heading">
            <Tag size={10} stroke-width={1.5} aria-hidden="true" />
            <span>{t("artist.tags")}</span>
          </h2>
          <div class="chips">
            {#each info.tags as tag}
              <span class="chip faint">{tag}</span>
            {/each}
          </div>
        </div>
      {/if}

      {#if info.disambiguation}
        <div class="section">
          <h2 class="heading">{t("artist.note")}</h2>
          <p class="list">{info.disambiguation}</p>
        </div>
      {/if}

      {#if !hasBio && info.genres.length === 0 && info.tags.length === 0 && !info.disambiguation}
        <p class="muted">{t("artist.noInfo")}</p>
      {/if}
    {/if}
  </div>
</section>

<style>
  .artist-page {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .view {
    height: 100%;
    overflow-y: auto;
    scrollbar-width: none;
    padding: 32px 40px 48px;
    max-width: 560px;
  }

  .view::-webkit-scrollbar {
    display: none;
  }

  .close {
    position: absolute;
    top: 20px;
    right: 20px;
    width: 28px;
    height: 28px;
    border: 1px solid var(--line);
    background: var(--bg-raise);
    color: var(--faint);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transform: rotate(45deg);
    border-radius: var(--radius-sm);
    transition: color 0.15s, border-color 0.15s, background 0.15s;
  }

  .close:hover {
    color: var(--text);
    border-color: var(--line-strong);
  }

  .close :global(svg) {
    transform: rotate(-45deg);
  }

  .header {
    margin-bottom: 32px;
  }

  .name-block {
    margin-bottom: 0;
  }

  .name {
    font-size: 36px;
    font-weight: 600;
    letter-spacing: -0.025em;
    line-height: 1.1;
    margin: 0;
    color: var(--text);
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 16px;
    font-size: 13px;
    color: var(--faint);
    margin: 10px 0 0;
    letter-spacing: 0.01em;
  }

  .meta-item {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .meta-item :global(svg) {
    flex-shrink: 0;
    color: var(--faint);
  }

  .section {
    margin-bottom: 28px;
  }

  .heading {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10px;
    font-weight: 500;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--faint);
    margin: 0 0 8px;
  }

  .heading :global(svg) {
    flex-shrink: 0;
    color: var(--faint);
  }

  .bio-clip {
    overflow: hidden;
    transition: max-height 320ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .bio {
    font-size: 14px;
    line-height: 1.7;
    color: var(--dim);
    margin: 0;
    white-space: pre-wrap;
  }

  .more {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-top: 10px;
    padding: 3px 9px 3px 0;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.04em;
    cursor: pointer;
    transition: color 0.15s;
  }

  .more:hover {
    color: color-mix(in srgb, var(--accent) 75%, white);
  }

  .more:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--accent-soft), 0 0 0 1px var(--accent);
  }

  .more :global(.more-icon) {
    transition: transform 0.28s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .more[aria-expanded="true"] :global(.more-icon) {
    transform: rotate(180deg);
  }

  .list {
    font-size: 13px;
    color: var(--dim);
    margin: 0;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    font-size: 12px;
    padding: 3px 10px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line);
    color: var(--dim);
    white-space: nowrap;
  }

  .chip.faint {
    background: none;
    border-color: var(--line);
    color: var(--faint);
  }

  .loading-state,
  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 200px;
    text-align: center;
    color: var(--faint);
  }

  .spinner {
    width: 28px;
    height: 28px;
    border: 2px solid var(--line);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-bottom: 12px;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .muted {
    font-size: 13px;
    color: var(--faint);
    margin: 0;
  }

  .err {
    color: var(--red);
  }
</style>
