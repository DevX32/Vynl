<script lang="ts">
  import { Play, Shuffle, User, AlertCircle, LoaderCircle } from "lucide-svelte";
  import type { CatalogueAlbum, CatalogueArtist, CatalogueTrack, Page } from "@lib/types";
  import { cataloguePath, loadCatalogueArtist } from "@state/catalogue.svelte";
  import { requestPlay } from "@state/now-playing.svelte";
  import { setShuffle } from "@state/player.svelte";
  import { blobSrc, splitWikiAttribution } from "@lib/format";
  import { t } from "@lib/i18n";
  import { friendlyError } from "@lib/errors";
  import CatalogueTrackList from "./CatalogueTrackList.svelte";
  import CatalogueCard from "./CatalogueCard.svelte";
  import BioModal from "./BioModal.svelte";

  let {
    id,
    onNavigate,
  }: {
    id: string;
    onNavigate: (page: Page) => void;
  } = $props();

  let artist = $state<CatalogueArtist | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let loadToken = 0;
  let bioEl = $state<HTMLParagraphElement | null>(null);
  let bioClamped = $state(false);
  let bioOpen = $state(false);

  function checkBioClamp(): void {
    if (!bioEl) return;
    bioClamped = bioEl.scrollHeight > bioEl.clientHeight + 1;
  }

  $effect(() => {
    if (!bioEl) return;
    const observer = new ResizeObserver(checkBioClamp);
    observer.observe(bioEl);
    checkBioClamp();
    return () => observer.disconnect();
  });

  const topTracks = $derived(artist?.topTracks ?? []);

  interface EditionGroup {
    key: string;
    base: string;
    year: number | null;
    cover: string | null;
    items: CatalogueAlbum[];
  }

  const LANGUAGES =
    "telugu|tamil|kannada|malayalam|hindi|english|bengali|marathi|punjabi|urdu|sinhala|nepali|oriya|odia|gujarati|sanskrit|thai|chinese|korean|japanese|spanish|french|german|italian|portuguese";

  function baseKey(title: string): string {
    let text = title.toLowerCase();
    text = text.replace(/\([^)]*\)|\[[^\]]*\]/g, " ");
    text = text.replace(
      new RegExp(`[-–—]\\s*(${LANGUAGES})\\b`, "gi"),
      " ",
    );
    return text.replace(/[^a-z0-9]+/g, " ").trim();
  }

  function baseTitle(title: string): string {
    let text = title.replace(/\([^)]*\)|\[[^\]]*\]/g, " ");
    text = text.replace(
      new RegExp(`[-–—]\\s*(${LANGUAGES})\\b`, "gi"),
      " ",
    );
    text = text.replace(/\s{2,}/g, " ");
    text = text.replace(/(?:\s*[-–—·:]\s*)+$/g, "").trim();
    return text || title.trim();
  }

  function franchise(title: string): string {
    const base = baseTitle(title);
    return base.split("|")[0].trim() || base;
  }

  function franchiseKey(title: string): string {
    return (
      franchise(title)
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, " ")
        .trim() || baseKey(title)
    );
  }

  function groupEditions(releases: readonly CatalogueAlbum[]): EditionGroup[] {
    const map = new Map<string, EditionGroup>();
    for (const release of releases) {
      const key = franchiseKey(release.title) || release.id;
      const existing = map.get(key);
      if (existing) {
        existing.items.push(release);
        if (existing.cover === null && release.cover) {
          existing.cover = release.cover;
        }
        if (release.year !== null) {
          existing.year = Math.max(existing.year ?? 0, release.year);
        }
        continue;
      }
      map.set(key, {
        key,
        base: franchise(release.title),
        year: release.year,
        cover: release.cover,
        items: [release],
      });
    }
    return [...map.values()];
  }

  const albumGroups = $derived(groupEditions(artist?.albums ?? []));
  const singleGroups = $derived(groupEditions(artist?.singles ?? []));

  function defaultEdition(group: EditionGroup): CatalogueAlbum {
    return (
      group.items.find((item) => /telugu/i.test(item.title)) ?? group.items[0]
    );
  }

  async function load(artistId: string, force = false): Promise<void> {
    if (!artistId) return;
    const token = ++loadToken;
    loading = true;
    error = null;
    try {
      const result = await loadCatalogueArtist(artistId, force);
      if (token === loadToken) artist = result;
    } catch (err) {
      if (token === loadToken) {
        error = err instanceof Error ? err.message : String(err);
      }
    } finally {
      if (token === loadToken) loading = false;
    }
  }

  $effect(() => {
    void load(id);
  });

  function play(shuffle = false): void {
    if (topTracks.length === 0) return;
    if (shuffle) setShuffle(true);
    const first = shuffle
      ? topTracks[Math.floor(Math.random() * topTracks.length)]
      : topTracks[0];
    requestPlay(
      first.id,
      topTracks.map((track) => cataloguePath(track.id)),
    );
  }

  function playTrack(track: CatalogueTrack): void {
    requestPlay(
      track.id,
      topTracks.map((item) => cataloguePath(item.id)),
    );
  }
</script>

<div class="detail-page">
  {#if artist}
    <div class="hero">
      <div class="hero-cover">
        {#if artist.cover}
          <img use:blobSrc={artist.cover} alt="" draggable="false" />
        {:else}
          <div class="hero-cover-placeholder">
            <User size={30} stroke-width={1} />
          </div>
        {/if}
      </div>

      <div class="hero-meta">
        <div class="hero-title display">{artist.name}</div>
        {#if artist.description}
          {@const wiki = splitWikiAttribution(artist.description)}
          <div class="bio">
            <div class="bio-head">
              <span class="bio-label mono">{t("common.about")}</span>
              {#if bioClamped}
                <button
                  class="bio-toggle mono"
                  aria-haspopup="dialog"
                  onclick={() => (bioOpen = true)}
                >
                  {t("catalogue.showMore")}
                </button>
              {/if}
            </div>
            <p class="description" bind:this={bioEl}>
              {wiki.body}
            </p>
            {#if wiki.attribution}
              <span class="bio-source mono">{wiki.attribution}</span>
            {/if}
          </div>
        {/if}
      </div>

      <div class="hero-actions">
        <button
          class="play-big"
          onclick={() => play(false)}
          disabled={topTracks.length === 0}
          aria-label={t("common.playAll")}
        >
          <Play size={16} fill="currentColor" stroke-width={0} />
        </button>
        <button
          class="icon-btn round"
          onclick={() => play(true)}
          disabled={topTracks.length === 0}
          aria-label={t("common.shuffle")}
        >
          <Shuffle size={14} stroke-width={1.5} />
        </button>
      </div>
    </div>
  {/if}

  <div class="detail-scroll">
    {#if loading && !artist}
      <div class="state">
        <span class="spin"><LoaderCircle size={22} stroke-width={1.5} /></span>
        <span class="state-sub mono">{t("common.loading")}</span>
      </div>
    {:else if error && !artist}
      <div class="state" role="alert">
        <AlertCircle size={26} stroke-width={1.2} />
        <span class="state-title display">{t("catalogue.loadFailed")}</span>
        <span class="state-sub mono">{friendlyError(error)}</span>
        <button class="retry-btn" onclick={() => void load(id, true)}>
          {t("common.retry")}
        </button>
      </div>
    {:else if artist}
      {#snippet belowTable()}
        {#if albumGroups.length > 0}
          <section class="section">
            <div class="section-head">
              <span class="section-label">{t("catalogue.albums")}</span>
            </div>
            <div class="card-grid">
              {#each albumGroups as group (group.key)}
                {@const multi = group.items.length > 1}
                {@const subtitle = multi
                  ? `${group.year ? `${group.year} · ` : ""}${t("catalogue.editionCount", { n: group.items.length })}`
                  : group.year
                    ? String(group.year)
                    : ""}
                <CatalogueCard
                  title={multi ? group.base : group.items[0].title}
                  {subtitle}
                  cover={group.cover}
                  kind="album"
                  badge={multi ? `×${group.items.length}` : null}
                  openLabel={t("catalogue.openAlbum")}
                  onopen={() =>
                    onNavigate(`catalogue:album:${defaultEdition(group).id}`)}
                />
              {/each}
            </div>
          </section>
        {/if}

        {#if singleGroups.length > 0}
          <section class="section">
            <div class="section-head">
              <span class="section-label">{t("catalogue.singles")}</span>
            </div>
            <div class="card-grid">
              {#each singleGroups as group (group.key)}
                {@const multi = group.items.length > 1}
                {@const subtitle = multi
                  ? `${group.year ? `${group.year} · ` : ""}${t("catalogue.editionCount", { n: group.items.length })}`
                  : group.year
                    ? String(group.year)
                    : ""}
                <CatalogueCard
                  title={multi ? group.base : group.items[0].title}
                  {subtitle}
                  cover={group.cover}
                  kind="album"
                  badge={multi ? `×${group.items.length}` : null}
                  openLabel={t("catalogue.openAlbum")}
                  onopen={() =>
                    onNavigate(`catalogue:album:${defaultEdition(group).id}`)}
                />
              {/each}
            </div>
          </section>
        {/if}
      {/snippet}

      {#if topTracks.length > 0}
        <CatalogueTrackList
          tracks={topTracks}
          showAlbum={true}
          onPlayTrack={playTrack}
          onNavigateAlbum={(albumId) => onNavigate(`catalogue:album:${albumId}`)}
        >
          {#snippet children()}
            {@render belowTable()}
          {/snippet}
        </CatalogueTrackList>
      {:else}
        <div class="below-scroll">
          {@render belowTable()}
        </div>
      {/if}
    {/if}
  </div>
</div>

{#if bioOpen && artist?.description}
  <BioModal
    heading={t("common.about")}
    text={artist.description}
    image={artist.cover}
    subtitle={artist.name}
    onClose={() => (bioOpen = false)}
  />
{/if}

<style>
  .detail-page {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .detail-scroll {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    gap: 12px;
  }

  .below-scroll {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-bottom: 36px;
    padding-right: 2px;
  }
.hero {
    display: flex;
    align-items: center;
    gap: 20px;
    padding: 24px 0 16px;
    padding-right: 10px;
    flex-wrap: wrap;
    flex-shrink: 0;
  }

  .hero-cover {
    width: 150px;
    height: 150px;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg-raise);
    box-shadow:
      0 0 0 1px rgba(255, 255, 255, 0.07),
      var(--shadow-md);
  }

  .hero-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .hero-cover-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--faint);
    background: var(--placeholder-gradient);
  }

  .hero-meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
  }

  .hero-title {
    font-size: 38px;
    line-height: 1.05;
    letter-spacing: -0.02em;
    overflow-wrap: anywhere;
  }

  .hero-actions {
    display: flex;
    align-items: center;
    gap: 18px;
    flex-shrink: 0;
  }

  .play-big {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent);
    color: var(--accent-text);
    border: none;
    cursor: pointer;
    transform: rotate(45deg);
    transition: transform 0.15s, background 0.15s;
  }

  .play-big :global(svg) {
    transform: rotate(-45deg);
  }

  .play-big:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 82%, #ffffff);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .play-big:active:not(:disabled) {
    transform: rotate(45deg) scale(0.92);
  }

  .play-big:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .icon-btn.round {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line-strong);
    background: var(--bg-raise);
    color: var(--dim);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transform: rotate(45deg);
    transition: color 0.15s, border-color 0.15s, background 0.15s;
  }

  .icon-btn.round :global(svg) {
    transform: rotate(-45deg);
  }

  .icon-btn.round:hover:not(:disabled) {
    color: var(--text);
    background: var(--surface);
  }

  .icon-btn.round:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .bio {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-width: 720px;
    flex-shrink: 0;
  }

  .bio-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .bio-label {
    font-size: 10px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--faint);
  }

  .bio-toggle {
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--faint);
    cursor: pointer;
    padding: 0;
    border: none;
    background: none;
    transition: color 0.12s;
  }

  .bio-toggle:hover {
    color: var(--text);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .bio-source {
    font-size: 9.5px;
    line-height: 1.5;
    letter-spacing: 0.02em;
    color: var(--faint);
    user-select: text;
    overflow-wrap: anywhere;
  }

  .description {
    margin: 0;
    font-size: 12px;
    line-height: 1.6;
    color: var(--dim);
    white-space: pre-line;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    overflow-wrap: anywhere;
  }

  .section {
    margin-bottom: 26px;
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 8px;
    padding: 8px 0;
  }

  .section-label {
    font-size: 12px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--dim);
  }

  .card-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(128px, 1fr));
    gap: 10px;
  }

  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    min-height: 320px;
    padding: 40px 20px;
    text-align: center;
    color: var(--faint);
  }

  .state-title {
    font-size: 23px;
    color: var(--dim);
  }

  .state-sub {
    font-size: 11px;
    letter-spacing: 0.04em;
    max-width: 420px;
    overflow-wrap: anywhere;
  }

  .retry-btn {
    margin-top: 4px;
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line-strong);
    background: var(--bg-raise);
    color: var(--dim);
    font-size: 11px;
    cursor: pointer;
  }

  .retry-btn:hover {
    color: var(--text);
  }

  .spin {
    display: inline-flex;
    align-items: center;
    animation: spin 0.85s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @container player (max-width: 620px) {
    .hero {
      align-items: center;
      gap: 14px;
      padding-top: 16px;
    }

    .hero-cover {
      width: 112px;
      height: 112px;
    }

    .hero-title {
      font-size: 30px;
    }
  }

  @container player (max-width: 420px) {
    .hero-cover {
      width: 92px;
      height: 92px;
    }

    .hero-title {
      font-size: 24px;
    }

    .hero-actions {
      gap: 12px;
    }
  }
</style>
