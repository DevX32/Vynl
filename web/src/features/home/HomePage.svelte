<script lang="ts">
  import { Play, Shuffle, Link2, Music2, Library } from "lucide-svelte";
  import { getLibrary } from "@state/library.svelte";
  import { requestPlay } from "@state/now-playing.svelte";
  import { setShuffle } from "@state/player.svelte";
  import { getPluginHomeSections } from "@state/plugins.svelte";
  import { vynl } from "@lib/vynl";
  import { blobSrc } from "../../lib/format";
  import { QUOTES } from "@lib/constants";
  import { t } from "@lib/i18n";
  import { getCurrentSettings } from "@state/settings.svelte";

  let {
    onNavigate,
  }: {
    onNavigate: (page: "player" | "vault" | "library") => void;
  } = $props();

  const library = $derived(getLibrary());
  const pluginSections = $derived(getPluginHomeSections());

  let _tick = $state(0);

  const greeting = $derived.by(() => {
    void _tick;
    const h = new Date().getHours();
    let base: string;
    if (h < 6) base = t("home.greeting.night");
    else if (h < 12) base = t("home.greeting.morning");
    else if (h < 18) base = t("home.greeting.afternoon");
    else base = t("home.greeting.evening");

    const name = getCurrentSettings().displayName?.trim();
    return name ? t("home.greeting.named", { greeting: base, name }) : base;
  });

  const quote = $derived.by(() => {
    void _tick;
    return QUOTES[(new Date().getDate() - 1) % QUOTES.length];
  });

  $effect(() => {
    void _tick;
    const now = new Date();
    const msUntilMidnight =
      new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1).getTime() -
      now.getTime();
    const timer = setTimeout(() => {
      _tick++;
    }, msUntilMidnight);
    return () => clearTimeout(timer);
  });

  const recentTracks = $derived(
    [...library]
      .sort((a, b) => {
        const aAdded = a.addedAt ?? a.mtime ?? 0;
        const bAdded = b.addedAt ?? b.mtime ?? 0;
        return (
          bAdded - aAdded ||
          (b.mtime ?? 0) - (a.mtime ?? 0) ||
          b.id.localeCompare(a.id)
        );
      })
      .slice(0, 6),
  );

  let featuredTrack = $state<typeof library[number] | null>(null);

  $effect(() => {
    const lib = library;
    if (lib.length > 0) {
      featuredTrack = lib[Math.floor(Math.random() * lib.length)];
    } else {
      featuredTrack = null;
    }
  });

  const artistCount = $derived(
    new Set(library.map((track) => track.artist)).size,
  );

  function shuffleAll(): void {
    if (library.length === 0) return;
    setShuffle(true);
    const first = library[Math.floor(Math.random() * library.length)];
    requestPlay(
      first.id,
      library.map((track) => track.path),
    );
  }

  function playTrack(id: string): void {
    requestPlay(
      id,
      library.map((track) => track.path),
    );
  }
</script>

<section class="home-page">
  <div class="home-scroll">
    <header class="home-hdr">
      <span class="greeting display">{greeting}</span>
      <span class="quote">{quote}</span>
      {#if library.length > 0}
        <span class="stat-line">
          {t("home.trackCount", {
            n: library.length,
            s: library.length === 1 ? "" : "s",
          })}
          {#if artistCount > 0}
            · {t("home.artistCount", {
              n: artistCount,
              s: artistCount === 1 ? "" : "s",
            })}
          {/if}
        </span>
      {/if}
    </header>

    {#if library.length > 0}
      {#if featuredTrack}
        <button
          class="featured"
          onclick={() => playTrack(featuredTrack!.id)}
          aria-label={t("home.jumpBackIn")}
        >
          <div class="featured-art">
            {#if featuredTrack.cover}
              <img
                class="featured-art-img"
                use:blobSrc={featuredTrack.cover}
                alt=""
                draggable="false"
              />
            {:else}
              <div class="featured-art-placeholder">
                <Music2 size={40} stroke-width={1} />
              </div>
            {/if}
          </div>
          <div class="featured-scrim"></div>
          <div class="featured-body">
            <span class="featured-label">{t("home.jumpBackIn")}</span>
            <span class="featured-title">{featuredTrack.title}</span>
            <span class="featured-artist">{featuredTrack.artist}</span>
          </div>
          <div class="featured-play">
            <Play size={20} fill="currentColor" stroke-width={0} />
          </div>
        </button>
      {/if}

      <div class="actions">
        <button class="action-btn" onclick={shuffleAll}>
          <Shuffle size={16} stroke-width={1.5} />
          <span>{t("home.shuffleAll")}</span>
        </button>
        <button class="action-btn" onclick={() => onNavigate("vault")}>
          <Link2 size={16} stroke-width={1.5} />
          <span>{t("home.addMusic")}</span>
        </button>
          <button class="action-btn" onclick={() => onNavigate("library")}>
            <Library size={16} stroke-width={1.5} />
            <span>{t("nav.library")}</span>
          </button>
      </div>

      {#if recentTracks.length > 0}
        <div class="section">
          <div class="section-head">
            <span class="section-label">{t("home.recentlyAdded")}</span>
            <button class="see-all" onclick={() => onNavigate("library")}
              >{t("home.viewAll")}</button
            >
          </div>
          <div class="recent-grid">
            {#each recentTracks as track (track.id)}
              <button class="recent-card" onclick={() => playTrack(track.id)}>
                <div class="recent-art">
                  {#if track.cover}
                    <img
                      class="recent-art-img"
                      use:blobSrc={track.cover}
                      alt=""
                      loading="lazy"
                      draggable="false"
                    />
                  {:else}
                    <div class="recent-art-placeholder">
                      <Music2 size={18} stroke-width={1.2} />
                    </div>
                  {/if}
                  <div class="recent-play">
                    <Play size={14} fill="var(--text)" stroke-width={0} />
                  </div>
                </div>
                <span class="recent-title">{track.title}</span>
                <span class="recent-artist">{track.artist}</span>
              </button>
            {/each}
          </div>
        </div>
      {/if}
    {:else}
        <div class="empty-state">
        <Music2 size={32} stroke-width={1} />
        <span class="empty-title display">{t("home.emptyTitle")}</span>
        <span class="empty-sub">{t("home.emptySub")}</span>
        <button class="action-btn" onclick={() => onNavigate("vault")}>
          <Link2 size={16} stroke-width={1.5} />
          <span>{t("home.openVault")}</span>
        </button>
      </div>
    {/if}

    {#if pluginSections.length > 0}
      {#each pluginSections as section (`${section.pluginId}:${section.id}`)}
        <div class="section">
          <div class="section-head">
            <span class="section-label">{section.title}</span>
            {#if section.pluginName}
              <span class="plugin-by">{section.pluginName}</span>
            {/if}
          </div>
          <div class="plugin-list">
            {#each section.items as item (item.id)}
              {#if item.trackId}
                <button
                  class="plugin-item"
                  onclick={() => item.trackId && playTrack(item.trackId)}
                >
                  <span class="plugin-item-title">{item.title}</span>
                  {#if item.subtitle}
                    <span class="plugin-item-sub">{item.subtitle}</span>
                  {/if}
                </button>
              {:else if item.url}
                <button
                  class="plugin-item"
                  onclick={() => item.url && void vynl.openExternal(item.url)}
                >
                  <span class="plugin-item-title">{item.title}</span>
                  {#if item.subtitle}
                    <span class="plugin-item-sub">{item.subtitle}</span>
                  {/if}
                </button>
              {:else}
                <div class="plugin-item">
                  <span class="plugin-item-title">{item.title}</span>
                  {#if item.subtitle}
                    <span class="plugin-item-sub">{item.subtitle}</span>
                  {/if}
                </div>
              {/if}
            {/each}
          </div>
        </div>
      {/each}
    {/if}
  </div>
</section>

<style>
  .home-page {
    height: 100%;
    overflow: hidden;
    overflow-x: hidden;
    contain: layout;
  }

  .home-scroll {
    height: 100%;
    overflow-y: auto;
    scrollbar-width: none;
    padding: 28px 0 40px;
  }

  .home-scroll::-webkit-scrollbar {
    display: none;
  }

  .home-hdr {
    display: flex;
    flex-direction: column;
    margin-bottom: 28px;
  }

  .greeting {
    font-size: 30px;
    line-height: 1.2;
    color: var(--text);
    margin-bottom: 6px;
  }

  .quote {
    font-size: 12px;
    font-style: italic;
    color: var(--faint);
  }

  .stat-line {
    font-size: 11px;
    color: var(--faint);
    margin-top: 10px;
  }

  .featured {
    position: relative;
    display: block;
    width: 100%;
    height: 168px;
    border-radius: var(--radius-sm);
    overflow: hidden;
    text-align: left;
    margin-bottom: 20px;
    background: var(--bg-raise);
    border: 1px solid var(--line);
  }

  .featured-art {
    position: absolute;
    inset: 0;
  }

  .featured-art-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: saturate(1.25) brightness(0.9) blur(2px);
    transition: filter 0.4s ease;
  }

  .featured:hover .featured-art-img {
    filter: saturate(1.35) brightness(0.95) blur(2px);
  }

  .featured-art-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--faint);
    background: var(--placeholder-gradient);
  }

  .featured-scrim {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background:
      linear-gradient(
        180deg,
        transparent 0%,
        transparent 28%,
        rgba(8, 8, 12, 0.28) 55%,
        rgba(8, 8, 12, 0.72) 82%,
        rgba(8, 8, 12, 0.9) 100%
      ),
      radial-gradient(
        ellipse 70% 90% at 0% 100%,
        rgba(0, 0, 0, 0.45) 0%,
        transparent 65%
      );
  }

  .featured-body {
    position: absolute;
    left: 20px;
    right: 76px;
    bottom: 18px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .featured-label {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.75);
    margin-bottom: 4px;
  }

  .featured-title {
    font-size: 18px;
    font-weight: 600;
    color: #fff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .featured-artist {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.7);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .featured-play {
    position: absolute;
    right: 18px;
    bottom: 18px;
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

  .featured-play :global(svg) {
    transform: rotate(-45deg);
  }

  .featured:hover .featured-play {
    background: color-mix(in srgb, var(--accent) 82%, #ffffff);
    transform: rotate(45deg) scale(1.08);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .featured:active .featured-play {
    transform: rotate(45deg) scale(0.92);
  }

  .section {
    margin-bottom: 28px;
  }

  .section-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  .section-label {
    font-size: 13px;
    color: var(--dim);
  }

  .see-all {
    font-size: 11px;
    color: var(--faint);
    transition: color 0.15s;
  }

  .see-all:hover {
    color: var(--text);
  }

  .plugin-by {
    font-size: 10.5px;
    color: var(--faint);
    font-family: var(--font-mono);
    letter-spacing: 0.06em;
  }

  .plugin-list {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .plugin-item {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    padding: 11px 14px;
    background: var(--bg-raise);
    border-top: 1px solid var(--line);
    text-align: left;
    transition: background 0.15s;
  }

  .plugin-item:first-child {
    border-top: none;
  }

  button.plugin-item:hover {
    background: var(--bg-hover, rgba(255, 255, 255, 0.04));
  }

  .plugin-item-title {
    font-size: 12.5px;
    color: var(--text);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .plugin-item-sub {
    font-size: 11px;
    color: var(--faint);
    font-family: var(--font-mono);
    flex-shrink: 0;
  }

  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 28px;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 16px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line);
    font-size: 12px;
    color: var(--dim);
    transition:
      background 0.15s,
      color 0.15s,
      border-color 0.15s;
  }

  .action-btn:hover {
    background: var(--surface);
    color: var(--text);
    border-color: var(--line-strong);
  }

  .recent-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }

  .recent-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px;
    border-radius: var(--radius-sm);
    text-align: left;
    flex: 1 1 120px;
    max-width: 160px;
    min-width: 100px;
    transition: background 0.15s, flex-basis 0.2s ease, max-width 0.2s ease;
  }

  .recent-card:hover {
    background: var(--bg-raise);
  }

  .recent-art {
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg-raise);
  }

  .recent-art-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .recent-art-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--faint);
    background: var(--placeholder-gradient);
  }

  .recent-play {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.45);
    opacity: 0;
    transition: opacity 0.15s;
  }

  .recent-card:hover .recent-play {
    opacity: 1;
  }

  .recent-title {
    font-size: 12px;
    line-height: 1.3;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text);
  }

  .recent-artist {
    font-size: 10.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--faint);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 60px 20px;
    text-align: center;
    color: var(--faint);
  }

  .empty-title {
    font-size: 18px;
    color: var(--dim);
  }

  .empty-sub {
    font-size: 12px;
    color: var(--faint);
    max-width: 280px;
  }

  @container player (max-width: 620px) {
    .home-scroll {
      padding: 20px 0 32px;
    }

    .home-hdr {
      margin-bottom: 20px;
    }

    .greeting {
      font-size: 24px;
    }

    .featured {
      height: 140px;
      margin-bottom: 16px;
    }
  }

  @container player (max-width: 420px) {
    .greeting {
      font-size: 20px;
    }

    .empty-state {
      padding: 32px 16px;
    }
  }
</style>