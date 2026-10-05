<script lang="ts">
  import { blobSrc } from "@lib/format";

  let {
    path,
    alt = "",
    showGlow = false,
  }: {
    path: string | null | undefined;
    alt?: string;
    showGlow?: boolean;
  } = $props();
</script>

<div class="art-backdrop" aria-hidden="true">
  <img class="art-backdrop-img" use:blobSrc={path} {alt} />
  {#if showGlow}
    <div class="art-backdrop-glow"></div>
  {/if}
  <div class="art-backdrop-vignette"></div>
</div>
<div class="art-grain" aria-hidden="true"></div>

<style>
  .art-backdrop {
    position: absolute;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    pointer-events: none;
    background: var(--bg);
  }

  .art-backdrop-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: blur(60px) saturate(1.6) brightness(0.35) contrast(1.1);
  }

  .art-backdrop-glow {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(
        ellipse 80% 50% at 50% 35%,
        var(--accent-glow, rgba(168, 148, 232, 0.3)) 0%,
        transparent 60%
      ),
      radial-gradient(
        ellipse 60% 40% at 50% 100%,
        rgba(0, 0, 0, 0.7) 0%,
        transparent 60%
      );
    opacity: 0.15;
    mix-blend-mode: screen;
  }

  .art-backdrop-vignette {
    position: absolute;
    inset: 0;
    background: radial-gradient(
      ellipse 70% 60% at 50% 45%,
      transparent 0%,
      rgba(14, 14, 16, 0.5) 70%,
      rgba(14, 14, 16, 0.85) 100%
    );
  }

  .art-grain {
    position: absolute;
    inset: 0;
    z-index: 1;
    pointer-events: none;
    opacity: 0.04;
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
    background-repeat: repeat;
    background-size: 180px;
  }
</style>
