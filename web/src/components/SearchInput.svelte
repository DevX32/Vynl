<script lang="ts">
  import { Search } from "lucide-svelte";
  import { t } from "@lib/i18n";

  let {
    value = $bindable(""),
    placeholder = "",
    oninput,
  }: {
    value?: string;
    placeholder?: string;
    oninput: (value: string) => void;
  } = $props();

  let inputEl: HTMLInputElement | undefined = $state();

  export function focus(): void {
    inputEl?.focus();
  }
</script>

<div class="search-input-wrap">
  <Search size={13} stroke-width={1.5} />
  <input
    bind:this={inputEl}
    class="search-field"
    type="text"
    placeholder={placeholder || t("search.placeholder")}
    {value}
    oninput={(e) => oninput(e.currentTarget.value)}
    spellcheck={false}
  />
</div>

<style>
  .search-input-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-raise);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 7px 12px;
    transition:
      border-color 0.15s,
      background 0.15s;
  }

  .search-input-wrap:focus-within {
    background: var(--bg-raise);
  }

  .search-input-wrap :global(svg) {
    flex-shrink: 0;
    color: var(--faint);
  }

  .search-field {
    width: 160px;
    background: none;
    border: none;
    outline: none;
    font-size: 12px;
    color: var(--text);
  }

  .search-field::placeholder {
    color: var(--faint);
  }

</style>
