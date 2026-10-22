<script lang="ts">
  import { getSmoothTime } from "@state/media-clock.svelte";
import {
    getLyricsActiveWordIndex,
    wordFill,
    type LyricLine,
    type Word,
  } from "@state/lyrics.svelte";

  let {
    words,
    lineEnd = undefined,
  }: {
    words: Word[];
    lineEnd?: LyricLine["end"];
  } = $props();

  let reduced = $state(false);

  $effect(() => {
    if (typeof matchMedia !== "function") return;
    const mq = matchMedia("(prefers-reduced-motion: reduce)");
    const sync = (): void => {
      reduced = mq.matches;
    };
    sync();
    mq.addEventListener("change", sync);
    return () => mq.removeEventListener("change", sync);
  });

  const t = $derived(reduced ? 0 : getSmoothTime());
  const sungIndex = $derived(getLyricsActiveWordIndex());

  function fillFor(i: number): number {
    if (reduced) return i <= sungIndex ? 1 : 0;
    return wordFill(words, i, t, lineEnd);
  }
</script>

<span class="karaoke">
  {#each words as word, i (i)}
    {@const fill = fillFor(i)}
    <span class="kw" class:started={fill > 0} class:bg={!!word.bg}>
      <span
        class="wipe"
        style:--fill-pct={`${Math.round(fill * 1000) / 10}%`}
      >{word.text}</span></span>{#if i < words.length - 1 && (word.sep ?? true)}{" "}{/if}
  {/each}
</span>

<style>
  .karaoke {
    display: inline;
  }

  .kw {
    display: inline-block;
    vertical-align: baseline;
    white-space: nowrap;
    --wipe-unlit: color-mix(in srgb, var(--dim) 70%, var(--bg));
    --wipe-lit: var(--text);
    transform: translate3d(0, 0, 0);
    transition: transform 0.35s var(--ease-out);
  }

  .kw.started {
    transform: translate3d(0, -3px, 0);
  }

  .wipe {
    /* Vertical padding extends the background box past the font's ascent/
       descent so background-clip:text still colors tall script marks
       (devanagari matras) that paint outside the em box. */
    padding: 0.4em 0;
    background-color: var(--wipe-unlit);
    background-repeat: no-repeat;
    background-position: left center;
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
  }

  .kw:not(.started) .wipe {
    background-image: none;
  }

  .kw.started .wipe {
    background-image: linear-gradient(
      90deg,
      var(--wipe-lit) 0%,
      var(--wipe-lit) calc(100% - 0.5em),
      transparent 100%
    );
    background-size: calc(var(--fill-pct) + 0.5em) 100%;
  }

  .kw.bg {
    font-style: italic;
    --wipe-unlit: color-mix(in srgb, var(--faint) 70%, var(--bg));
    --wipe-lit: color-mix(in srgb, var(--dim) 60%, #888);
  }

  @media (prefers-reduced-motion: reduce) {
    .kw {
      transition: none;
    }
  }
</style>