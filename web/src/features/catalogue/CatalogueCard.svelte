<script lang="ts">
  import { Disc3, ListMusic } from "lucide-svelte";
  import { blobSrc } from "@lib/format";

  let {
    title,
    subtitle = "",
    cover = null,
    kind = "album",
    badge = null,
    onopen,
    openLabel,
  }: {
    title: string;
    subtitle?: string;
    cover?: string | null;
    kind?: "album" | "playlist";
    badge?: string | null;
    onopen: () => void;
    openLabel: string;
  } = $props();
</script>

<button class="card" onclick={onopen} aria-label={`${openLabel}: ${title}`}>
  <span class="art">
    {#if cover}
      <img
        class="art-img"
        use:blobSrc={cover}
        alt=""
        loading="lazy"
        draggable="false"
      />
    {:else}
      <span class="art-placeholder">
        {#if kind === "playlist"}
          <ListMusic size={26} stroke-width={1} />
        {:else}
          <Disc3 size={26} stroke-width={1} />
        {/if}
      </span>
    {/if}
    {#if badge}
      <span class="art-badge mono">{badge}</span>
    {/if}
  </span>
  <span class="meta">
    <span class="title">{title}</span>
    {#if subtitle}
      <span class="subtitle">{subtitle}</span>
    {/if}
  </span>
</button>

<style>
  .card {
    display: flex;
    flex-direction: column;
    min-width: 0;
    padding: 5px;
    border-radius: var(--radius-sm);
    background: none;
    border: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s;
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
    box-shadow: var(--shadow-sm);
  }

  .art-badge {
    position: absolute;
    right: 6px;
    bottom: 6px;
    font-size: 10px;
    letter-spacing: 0.04em;
    color: var(--text);
    background: rgba(10, 10, 12, 0.78);
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: var(--radius-sm);
    padding: 2px 6px;
    backdrop-filter: blur(6px);
    pointer-events: none;
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
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    overflow-wrap: anywhere;
    line-height: 1.35;
    min-height: 2.7em;
  }

  .subtitle {
    font-size: 11px;
    color: var(--faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
