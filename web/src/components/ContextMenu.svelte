<script lang="ts">
  import { Pencil, Trash2, Plus } from "lucide-svelte";
  import { t } from "@lib/i18n";

  export interface CtxItem {
    label: string;
    icon?: typeof import("lucide-svelte").Pencil;
    danger?: boolean;
    disabled?: boolean;
    secondary?: string;
    action: () => void;
  }

  export type CtxEntry =
    | CtxItem
    | { type: "separator" }
    | { label?: string; items: CtxItem[] };

  let {
    x = 0,
    y = 0,
    onclose,
    onrename,
    ondelete,
    onnewplaylist,
    items,
  }: {
    x: number;
    y: number;
    onclose: () => void;
    onrename?: () => void;
    ondelete?: () => void;
    onnewplaylist?: () => void;
    items?: CtxEntry[];
  } = $props();

  let menuEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (!menuEl) return;
    const rect = menuEl.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    if (rect.right > vw) menuEl.style.left = `${x - rect.width}px`;
    if (rect.bottom > vh) menuEl.style.top = `${y - rect.height}px`;
    if (rect.left < 0) menuEl.style.left = `0px`;
    if (rect.top < 0) menuEl.style.top = `0px`;
    const first = menuEl.querySelector<HTMLElement>("[role=menuitem]:not([aria-disabled='true'])");
    first?.focus();
  });

  function handleClickOutside(e: MouseEvent) {
    if (menuEl && !menuEl.contains(e.target as Node)) {
      onclose();
    }
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      onclose();
      return;
    }
    if (!menuEl) return;
    const items = Array.from(
      menuEl.querySelectorAll<HTMLElement>("[role=menuitem]:not([aria-disabled='true'])"),
    );
    if (items.length === 0) return;
    const idx = items.indexOf(document.activeElement as HTMLElement);
    if (e.key === "ArrowDown") {
      e.preventDefault();
      items[(idx + 1) % items.length]?.focus();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      items[(idx - 1 + items.length) % items.length]?.focus();
    } else if (e.key === "Tab") {
      e.preventDefault();
      onclose();
    }
  }

  function getItemClass(item: CtxItem) {
    let cls = "ctx-item";
    if (item.danger) cls += " ctx-danger";
    return cls;
  }
</script>

<svelte:window onclick={handleClickOutside} onkeydown={handleKey} />

<div
  class="ctx-menu"
  bind:this={menuEl}
  style:left="{x}px"
  style:top="{y}px"
  role="menu"
>
  {#if onnewplaylist}
    <button
      class="ctx-item"
      role="menuitem"
      onclick={() => {
        onnewplaylist();
        onclose();
      }}
    >
      <Plus size={12} stroke-width={1.5} />
      <span>{t("contextMenu.newPlaylist")}</span>
    </button>
  {/if}
  {#if onrename}
    <button
      class="ctx-item"
      role="menuitem"
      onclick={() => {
        onrename();
        onclose();
      }}
    >
      <Pencil size={12} stroke-width={1.5} />
      <span>{t("contextMenu.rename")}</span>
    </button>
  {/if}
  {#if (onrename || onnewplaylist) && ondelete}
    <div class="ctx-sep"></div>
  {/if}
  {#if ondelete}
    <button
      class="ctx-item ctx-danger"
      role="menuitem"
      onclick={() => {
        ondelete();
        onclose();
      }}
    >
      <Trash2 size={12} stroke-width={1.5} />
      <span>{t("contextMenu.delete")}</span>
    </button>
  {/if}
  {#if items}
    {#if (onrename || onnewplaylist || ondelete) && items.length > 0}
      <div class="ctx-sep"></div>
    {/if}
    {#each items as entry, i (i)}
      {#if typeof entry === "object" && "type" in entry && entry.type === "separator"}
        <div class="ctx-sep"></div>
      {:else if "items" in entry}
        <div class="ctx-group">
          {#if entry.label}
            <div class="ctx-group-label">{entry.label}</div>
          {/if}
          {#each entry.items as item (item.label)}
            <button
              class={getItemClass(item)}
              class:ctx-disabled={item.disabled}
              role="menuitem"
              aria-disabled={item.disabled || undefined}
              onclick={() => {
                if (item.disabled) return;
                item.action();
                onclose();
              }}
            >
              {#if item.icon}
                {@const Icon = item.icon}
                <Icon size={12} stroke-width={1.5} />
              {/if}
              <span>{item.label}</span>
              {#if item.secondary}
                <span class="ctx-secondary">{item.secondary}</span>
              {/if}
            </button>
          {/each}
        </div>
      {:else}
        {@const e = entry as CtxItem}
        <button
          class={getItemClass(e)}
          class:ctx-disabled={e.disabled}
          role="menuitem"
          aria-disabled={e.disabled || undefined}
          onclick={() => {
            if (e.disabled) return;
            e.action();
            onclose();
          }}
        >
          {#if e.icon}
            {@const Icon = e.icon}
            <Icon size={12} stroke-width={1.5} />
          {/if}
          <span>{e.label}</span>
          {#if e.secondary}
            <span class="ctx-secondary">{e.secondary}</span>
          {/if}
        </button>
      {/if}
    {/each}
  {/if}
</div>

<style>
  .ctx-menu {
    position: fixed;
    min-width: 170px;
    max-width: 280px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 5px 4px;
    z-index: 999;
    box-shadow: var(--shadow-lg);
    animation: ctx-in 0.12s cubic-bezier(0.25, 0.08, 0.25, 1);
  }

  @keyframes ctx-in {
    from {
      opacity: 0;
      transform: scale(0.96) translateY(-4px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }

  .ctx-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    text-align: left;
    cursor: pointer;
    font-size: 12px;
    color: var(--text);
    transition: background 0.12s;
  }

  .ctx-item:hover {
    background: var(--bg-raise);
  }

  .ctx-item:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .ctx-danger {
    color: var(--red);
  }

  .ctx-danger:hover {
    background: color-mix(in srgb, var(--red) 10%, transparent);
  }

  .ctx-disabled {
    opacity: 0.4;
    cursor: default;
  }

  .ctx-disabled:hover {
    background: none;
  }

  .ctx-secondary {
    margin-left: auto;
    font-size: 10.5px;
    color: var(--faint);
    white-space: nowrap;
  }

  .ctx-sep {
    height: 1px;
    background: var(--line);
    margin: 4px 6px;
  }

  .ctx-group-label {
    font-size: 9px;
    letter-spacing: 0.16em;
    color: var(--faint);
    padding: 5px 10px 4px;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .ctx-group {
    padding: 2px 0;
  }

  .ctx-menu::before {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 20px;
    background: linear-gradient(
      to bottom,
      rgba(0, 0, 0, 0.15) 0%,
      transparent 100%
    );
    pointer-events: none;
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
  }
</style>
