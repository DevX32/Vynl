<script lang="ts">
  import { Music } from "lucide-svelte";
  import { blobSrc } from "@lib/format";
  import { getCurrentTrack, matchesCurrentSong } from "@state/now-playing.svelte";
  import type { CatalogueTrack } from "@lib/types";
  import { t } from "@lib/i18n";

  let {
    track,
    onplay,
    onmenu,
  }: {
    track: CatalogueTrack;
    onplay: () => void;
    onmenu?: (e: MouseEvent, track: CatalogueTrack) => void;
  } = $props();

  const isCurrent = $derived.by(() => {
    if (getCurrentTrack()?.id === track.id) return true;
    return matchesCurrentSong(
      track.title,
      track.artist,
      track.duration ?? 0,
    );
  });
</script>

<button
  class="card"
  class:playing={isCurrent}
  onclick={onplay}
  oncontextmenu={(e) => onmenu?.(e, track)}
  aria-label={`${t("player.play")}: ${track.title} ${track.artist}`}
>
  <span class="art">
    {#if track.cover}
      <img
        class="art-img"
        use:blobSrc={track.cover}
        alt=""
        loading="lazy"
        decoding="async"
        fetchpriority="low"
        draggable="false"
      />
    {:else}
      <span class="art-placeholder">
        <Music size={26} stroke-width={1} />
      </span>
    {/if}
  </span>
  <span class="meta">
    <span class="title" class:accent={isCurrent}>{track.title}</span>
    <span class="subtitle">{track.artist}</span>
  </span>
</button>

<style>
  .card {
    display: flex;
    flex-direction: column;
    width: 100%;
    min-width: 0;
    padding: 5px;
    border-radius: var(--radius-sm);
    background: none;
    border: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s;
    content-visibility: auto;
    contain-intrinsic-size: auto 180px;
  }

  .card:hover {
    background: var(--bg-raise);
  }

  .art {
    display: block;
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg-raise);
    border: 1px solid var(--line);
  }

  .art-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .art-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--faint);
    background: var(--placeholder-gradient);
    border: 1px solid var(--line);
  }

  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    padding: 7px 1px 3px;
  }

  .title {
    font-size: 12.5px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .title.accent {
    color: var(--accent);
  }

  .subtitle {
    font-size: 11px;
    color: var(--faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
