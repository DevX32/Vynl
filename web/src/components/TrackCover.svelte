<script lang="ts">
  import { blobSrc } from "@lib/format";

  let {
    src = null,
    eager = false,
    shadow,
  }: {
    src?: string | null;
    eager?: boolean;
    shadow?: string;
  } = $props();
</script>

<span class="track-cover" style:--cover-shadow={shadow}>
  {#if src}
    <img
      class="img"
      use:blobSrc={src}
      alt=""
      loading={eager ? "eager" : "lazy"}
      draggable="false"
    />
  {:else}
    <span class="ph"></span>
  {/if}
</span>

<style>
  .track-cover {
    display: block;
    flex-shrink: 0;
    inline-size: var(--cover-size, 40px);
    block-size: var(--cover-size, 40px);
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg-raise);
    box-shadow: var(--cover-shadow, 0 1px 4px rgba(0, 0, 0, 0.15));
  }

  .img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .ph {
    width: 100%;
    height: 100%;
    background: var(--placeholder-gradient);
    border: 1px solid var(--line);
  }
</style>
