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
    const sync = () => (reduced = mq.matches);
    sync();
    mq.addEventListener("change", sync);
    return () => mq.removeEventListener("change", sync);
  });

  const now = $derived(reduced ? 0 : getSmoothTime());
  const activeWord = $derived(getLyricsActiveWordIndex());

  function fill(i: number): number {
    if (reduced) return i <= activeWord ? 1 : 0;
    return wordFill(words, i, now, lineEnd);
  }
</script>

<span class="karaoke" aria-hidden="true">
  {#each words as word, i (i)}
    {@const pct = fill(i)}
    <span class="word" class:active={pct > 0} class:bg={!!word.bg}>
      <span
        class="wipe"
        style:--fill={`${Math.max(0, Math.min(100, pct * 100))}%`}
      >{word.text}</span></span
    >{#if i < words.length - 1 && (word.sep ?? true)}{" "}{/if}
  {/each}
</span>

<style>
  .karaoke {
    display: inline;
  }

  .word {
    display: inline-block;
    white-space: nowrap;
    vertical-align: baseline;
    --unlit: color-mix(in srgb, var(--dim) 68%, var(--bg));
    --lit: var(--text);
    transform: translate3d(0, 0, 0);
    transition: transform 320ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .word.active {
    transform: translate3d(0, -0.08em, 0);
  }

  .wipe {
    padding: 0.35em 0;
    color: transparent;
    background-image: linear-gradient(
      90deg,
      var(--lit) 0%,
      var(--lit) calc(var(--fill) - 0.6em),
      color-mix(in srgb, var(--lit) 55%, var(--unlit)) var(--fill),
      var(--unlit) calc(var(--fill) + 0.6em),
      var(--unlit) 100%
    );
    -webkit-background-clip: text;
    background-clip: text;
  }

  .word.bg {
    font-style: italic;
    --unlit: color-mix(in srgb, var(--faint) 72%, var(--bg));
    --lit: color-mix(in srgb, var(--dim) 70%, var(--text));
  }

  @media (prefers-reduced-motion: reduce) {
    .word {
      transition: none;
    }
  }
</style>