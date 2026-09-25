<script lang="ts">
  import { Search, Loader } from "lucide-svelte";
  import { t } from "@lib/i18n";

  let {
    value = $bindable(""),
    onResolve,
    onSearch,
    onClear,
    resolving,
    searching,
  }: {
    value?: string;
    onResolve: () => void;
    onSearch: (query: string) => void;
    onClear: () => void;
    resolving: boolean;
    searching: boolean;
  } = $props();

  let debounceId: ReturnType<typeof setTimeout> | null = null;
  let inputEl = $state<HTMLInputElement>();
  let lastScheduled = "";

  function clearDebounce() {
    if (debounceId != null) {
      clearTimeout(debounceId);
      debounceId = null;
    }
  }

  $effect(() => {
    return () => clearDebounce();
  });

  $effect(() => {
    if (!value.trim()) {
      clearDebounce();
      lastScheduled = "";
    }
  });

  function isUrl(v: string): boolean {
    return (
      /open\.spotify\.com\/(track|album|playlist)\//.test(v) ||
      /^spotify:(track|album|playlist):/.test(v)
    );
  }

  function schedule(v: string) {
    clearDebounce();
    const trimmed = v.trim();
    if (!trimmed) {
      lastScheduled = "";
      onClear();
      return;
    }
    lastScheduled = trimmed;
    const delay = isUrl(trimmed) ? 300 : 350;
    debounceId = setTimeout(() => {
      debounceId = null;
      if (value.trim() !== lastScheduled) return;
      if (isUrl(trimmed)) onResolve();
      else onSearch(trimmed);
    }, delay);
  }

  function handleClear() {
    clearDebounce();
    lastScheduled = "";
    value = "";
    onClear();
    setTimeout(() => inputEl?.focus(), 0);
  }

  function handleSubmit() {
    const trimmed = value.trim();
    if (!trimmed) return;
    clearDebounce();
    lastScheduled = trimmed;
    if (isUrl(trimmed)) onResolve();
    else onSearch(trimmed);
  }
</script>

<section class="hero">
  <div class="hero-copy">
    <div class="kicker mono">{t("vault.kicker")}</div>
    <h1 class="display">
      {t("vault.heroLine1")}
      <em class="accent-italic">{t("vault.heroItalic")}</em>
      {t("vault.heroLine2")}
    </h1>

    <div class="input-wrap" class:focused={!!value}>
      <div class="input-icon">
        {#if resolving || searching}
          <Loader size={15} stroke-width={2} class="spin" />
        {:else}
          <Search size={15} stroke-width={1.5} />
        {/if}
      </div>
      <input
        bind:this={inputEl}
        class="search-input"
        placeholder={t("vault.inputPlaceholder")}
        bind:value
        oninput={() => schedule(value)}
        spellcheck={false}
        onkeydown={(e) => {
          if (e.key === "Enter") handleSubmit();
          if (e.key === "Escape") handleClear();
        }}
      />
      {#if value}
        <button type="button" class="clear-btn" onclick={handleClear}>Clear</button>
      {/if}
    </div>
  </div>
</section>

<style>
  .hero {
    padding: 32px 0 8px;
  }

  .hero-copy {
    max-width: 580px;
  }

  .kicker {
    font-size: 10px;
    color: var(--faint);
    letter-spacing: 0.16em;
    text-transform: uppercase;
    margin-bottom: 14px;
  }

  h1 {
    margin: 0 0 24px;
    font-size: 40px;
    font-weight: 400;
    line-height: 1.08;
    letter-spacing: -0.01em;
    overflow-wrap: break-word;
    min-width: 0;
  }

  .accent-italic {
    font-style: italic;
    color: var(--accent);
  }

  .input-wrap {
    display: flex;
    align-items: center;
    gap: 0;
    background: var(--bg-raise);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 0 14px;
    height: 44px;
    transition:
      border-color 0.2s,
      box-shadow 0.2s;
  }

  .input-wrap.focused:not(:focus-within) {
    border-color: var(--line-strong);
  }

  .input-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    flex-shrink: 0;
    color: var(--faint);
  }

  .search-input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    outline: none;
    font-size: 13px;
    color: var(--text);
    padding: 0 10px;
    height: 100%;
  }

  .search-input::placeholder {
    color: var(--faint);
  }

  .clear-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 4px 10px;
    border-radius: var(--radius-sm);
    background: var(--line-strong);
    border: 1px solid transparent;
    outline: none;
    box-shadow: none;
    color: var(--dim);
    cursor: pointer;
    flex-shrink: 0;
    margin-left: 4px;
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.02em;
    transition:
      background 0.15s,
      color 0.15s;
  }

  .clear-btn:hover,
  .clear-btn:focus {
    color: var(--text);
    border-color: transparent;
    box-shadow: none;
  }

  .input-icon :global(.spin) {
    animation: spin 0.7s linear infinite;
    color: var(--accent);
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @container player (max-width: 620px) {
    .hero {
      padding: 20px 0 6px;
    }

    h1 {
      font-size: 32px;
      margin-bottom: 16px;
    }

    .input-wrap {
      height: 40px;
      padding: 0 10px;
    }

    .search-input {
      font-size: 12px;
    }
  }

  @container player (max-width: 420px) {
    .hero {
      padding: 14px 0 4px;
    }

    h1 {
      font-size: 26px;
      margin-bottom: 12px;
    }

    .input-wrap {
      height: 38px;
    }
  }
</style>
