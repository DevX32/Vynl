<script lang="ts" module>
  export type SelectOption = {
    value: string;
    label: string;
    disabled?: boolean;
  };

  let instanceCount = 0;
</script>

<script lang="ts">
  import { tick } from "svelte";
  import { ChevronDown } from "lucide-svelte";

  let {
    options,
    value = $bindable(),
    placeholder = "Select…",
    disabled = false,
    size = "md",
    label,
    onchange,
  }: {
    options: SelectOption[];
    value?: string;
    placeholder?: string;
    disabled?: boolean;
    size?: "sm" | "md";
    label?: string;
    onchange?: (value: string) => void;
  } = $props();

  const uid = `select-${instanceCount++}`;

  let open = $state(false);
  let activeIndex = $state(-1);
  let rootEl: HTMLDivElement | undefined = $state();
  let listEl: HTMLDivElement | undefined = $state();

  const selected = $derived(options.find((o) => o.value === value));
  const optionId = (i: number) => `${uid}-opt-${i}`;

  async function openList() {
    if (disabled || options.length === 0) return;
    open = true;
    activeIndex = Math.max(
      0,
      options.findIndex((o) => o.value === value),
    );
    await tick();
    scrollActiveIntoView();
  }

  function closeList() {
    open = false;
    activeIndex = -1;
  }

  function toggle() {
    if (open) closeList();
    else void openList();
  }

  function commit(opt: SelectOption) {
    if (opt.disabled) return;
    if (opt.value !== value) {
      value = opt.value;
      onchange?.(opt.value);
    }
    closeList();
  }

  function scrollActiveIntoView() {
    if (!listEl) return;
    const el = listEl.querySelector<HTMLElement>(`[data-index="${activeIndex}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }

  function moveActive(delta: number) {
    if (options.length === 0) return;
    let i = activeIndex;
    for (let step = 0; step < options.length; step++) {
      i = (i + delta + options.length) % options.length;
      if (!options[i]?.disabled) break;
    }
    activeIndex = i;
    scrollActiveIntoView();
  }

  function handleTriggerKey(e: KeyboardEvent) {
    if (disabled) return;
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        if (!open) void openList();
        else moveActive(1);
        break;
      case "ArrowUp":
        e.preventDefault();
        if (!open) void openList();
        else moveActive(-1);
        break;
      case "Enter":
      case " ":
        e.preventDefault();
        if (!open) {
          void openList();
        } else if (activeIndex >= 0) {
          commit(options[activeIndex]);
        }
        break;
      case "Escape":
        if (open) {
          e.preventDefault();
          closeList();
        }
        break;
      case "Home":
        if (open) {
          e.preventDefault();
          activeIndex = options.findIndex((o) => !o.disabled);
          scrollActiveIntoView();
        }
        break;
      case "End":
        if (open) {
          e.preventDefault();
          for (let i = options.length - 1; i >= 0; i--) {
            if (!options[i].disabled) {
              activeIndex = i;
              break;
            }
          }
          scrollActiveIntoView();
        }
        break;
      case "Tab":
        closeList();
        break;
    }
  }

  function handleClickOutside(e: MouseEvent) {
    if (open && rootEl && !rootEl.contains(e.target as Node)) closeList();
  }
</script>

<svelte:window onclick={handleClickOutside} />

<div class="select-root {size}" bind:this={rootEl}>
  <button
    type="button"
    class="select-trigger"
    class:open
    class:placeholder={!selected}
    {disabled}
    role="combobox"
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls="{uid}-list"
    aria-activedescendant={open && activeIndex >= 0 ? optionId(activeIndex) : undefined}
    aria-label={label}
    onclick={toggle}
    onkeydown={handleTriggerKey}
  >
    <span class="select-value">{selected ? selected.label : placeholder}</span>
    <ChevronDown size={14} stroke-width={1.5} />
  </button>

  {#if open}
    <div
      id="{uid}-list"
      class="select-list"
      role="listbox"
      bind:this={listEl}
      aria-label={label}
    >
      {#each options as opt, i}
        <!-- svelte-ignore a11y_interactive_supports_focus -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          id={optionId(i)}
          role="option"
          aria-selected={opt.value === value}
          aria-disabled={opt.disabled}
          data-index={i}
          class="select-option"
          class:active={i === activeIndex}
          class:selected={opt.value === value}
          class:disabled={opt.disabled}
          onclick={() => commit(opt)}
          onmouseenter={() => (activeIndex = i)}
        >
          <span class="select-option-label">{opt.label}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .select-root {
    position: relative;
    display: inline-flex;
    width: 100%;
  }

  .select-trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    background: transparent;
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 8px 8px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
    transition: color 0.15s;
  }

  .sm .select-trigger {
    padding: 2px 8px;
    font-size: 11px;
  }

  .select-trigger:focus {
    outline: none;
  }

  .select-trigger:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .select-value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
  }

  .select-trigger.placeholder .select-value {
    color: var(--faint);
  }

  .select-trigger :global(svg) {
    flex-shrink: 0;
    transition: transform 0.15s;
  }

  .select-trigger.open :global(svg) {
    transform: rotate(180deg);
  }

  .select-list {
    position: absolute;
    z-index: 100;
    top: calc(100% + 8px);
    left: 0;
    right: 0;
    margin: 0;
    padding: 6px 0;
    max-height: 240px;
    overflow-y: auto;
    scrollbar-width: none;
    background: var(--bg-raise);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-lg);
    animation: select-in 0.1s ease-out;
  }
 
  .select-list::-webkit-scrollbar {
    display: none;
  }

  @keyframes select-in {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  .select-option {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
    -webkit-tap-highlight-color: transparent;
    transition: background 0.1s, color 0.1s;
  }

  .sm .select-option {
    padding: 6px 10px;
    font-size: 11px;
  }

  .select-option-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .select-option:hover {
    background: var(--bg-raise);
  }

  .select-option.active {
    background: var(--bg-raise);
  }

  .select-option.selected {
    color: var(--accent);
    font-weight: 500;
  }

  .select-option.disabled {
    opacity: 0.35;
    cursor: default;
    pointer-events: none;
  }
</style>