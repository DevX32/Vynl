<script lang="ts" generics="T">
  import type { Snippet } from "svelte";
  import { untrack } from "svelte";

  let {
    items,
    scrollEl,
    row,
    rowKey,
    estimateRowHeight = 54,
    rowGap = 0,
    overscan = 6,
    class: className = "",
  }: {
    items: readonly T[];
    scrollEl: HTMLElement | null;
    row: Snippet<[T, number]>;
    rowKey: (item: T, index: number) => string;
    estimateRowHeight?: number;
    rowGap?: number;
    overscan?: number;
    class?: string;
  } = $props();

  let scrollTop = $state(0);
  let viewport = $state(0);
  let rowHeight = $state(untrack(() => estimateRowHeight));
  let rootEl: HTMLDivElement | undefined = $state();

  function read(): void {
    if (!scrollEl) return;
    scrollTop = scrollEl.scrollTop;
    viewport = scrollEl.clientHeight;
  }

  const pitch = $derived(rowHeight + rowGap);

  const totalHeight = $derived(
    items.length === 0 ? 0 : items.length * pitch - rowGap,
  );

  const range = $derived.by(() => {
    const len = items.length;
    if (len === 0) return { start: 0, end: 0 };

    if (viewport <= 0) {
      return { start: 0, end: Math.min(len, overscan * 2) };
    }

    const maxStart = Math.max(0, len - 1);
    const start = Math.min(
      maxStart,
      Math.max(0, Math.floor(scrollTop / pitch) - overscan),
    );
    const end = Math.min(
      len,
      Math.max(start + 1, Math.ceil((scrollTop + viewport) / pitch) + overscan),
    );
    return { start, end };
  });

  const visible = $derived(
    items.slice(range.start, range.end).map((item, i) => ({
      item,
      index: range.start + i,
    })),
  );

  $effect(() => {
    const el = scrollEl;
    if (!el) return;
    read();
    el.addEventListener("scroll", read, { passive: true });
    const ro = new ResizeObserver(read);
    ro.observe(el);
    return () => {
      el.removeEventListener("scroll", read);
      ro.disconnect();
    };
  });

  $effect(() => {
    const el = rootEl;
    if (!el) return;
    const ro = new ResizeObserver(() => read());
    ro.observe(el);
    return () => ro.disconnect();
  });

  $effect(() => {
    const first = visible[0] ? rootEl?.firstElementChild : undefined;
    if (!first) return;
    const ro = new ResizeObserver(() => {
      const h = first.getBoundingClientRect().height;
      if (h > 0 && Math.abs(h - rowHeight) > 0.5) rowHeight = h;
    });
    ro.observe(first);
    return () => ro.disconnect();
  });
</script>

<div class="vlist {className}" style:height="{totalHeight}px" bind:this={rootEl}>
  {#each visible as entry (rowKey(entry.item, entry.index))}
    <div
      class="vrow"
      data-index={entry.index}
      style:transform="translateY({entry.index * pitch}px)"
    >
      {@render row(entry.item, entry.index)}
    </div>
  {/each}
</div>

<style>
  .vlist {
    position: relative;
    flex-shrink: 0;
    width: 100%;
  }

  .vrow {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    box-sizing: border-box;
  }
</style>