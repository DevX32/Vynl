<script lang="ts">
  import { Search, Loader, Link2, ListChecks, FolderDown } from "lucide-svelte";
  import { t } from "@lib/i18n";

  let {
    value = $bindable(""),
    compact = false,
    onResolve,
    onSearch,
    onClear,
    resolving,
    searching,
  }: {
    value?: string;
    compact?: boolean;
    onResolve: () => void;
    onSearch: (query: string) => void;
    onClear: () => void;
    resolving: boolean;
    searching: boolean;
  } = $props();

  let debounceId: ReturnType<typeof setTimeout> | null = null;
  let inputEl = $state<HTMLInputElement>();
  let lastScheduled = "";

  const STEPS = [
    { icon: Link2, title: t("vault.step1Title"), body: t("vault.step1Body") },
    { icon: ListChecks, title: t("vault.step2Title"), body: t("vault.step2Body") },
    { icon: FolderDown, title: t("vault.step3Title"), body: t("vault.step3Body") },
  ];

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

<section class="hero" class:compact>
  <div class="hero-main">
    {#if !compact}
      <div class="kicker mono">{t("vault.kicker")}</div>
      <h1 class="display">
        {t("vault.heroLine1")}
        <em class="accent-italic">{t("vault.heroItalic")}</em>
        {t("vault.heroLine2")}
      </h1>
    {/if}

    <div class="input-wrap" class:focused={!!value}>
      <div class="input-icon">
        {#if resolving || searching}
          <Loader size={16} stroke-width={2} class="spin" />
        {:else}
          <Search size={16} stroke-width={1.5} />
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
    </div>

    {#if !compact}
      <div class="hint mono">
        <span class="key">{t("vault.keyEnter")}</span>
        <span>{t("vault.hintSearch")}</span>
        <span class="hint-sep"></span>
        <span class="key">{t("vault.keyEscape")}</span>
        <span>{t("vault.hintClear")}</span>
      </div>
    {/if}
  </div>

  {#if !compact}
    <aside class="steps-panel">
      <div class="panel-label mono">{t("vault.howItWorks")}</div>

      <ol class="steps">
        {#each STEPS as step, i (step.title)}
          {@const Icon = step.icon}
          <li class="step">
            <span class="step-index mono">{String(i + 1).padStart(2, "0")}</span>
            <span class="step-icon"><Icon size={14} stroke-width={1.5} /></span>
            <span class="step-copy">
              <span class="step-title">{step.title}</span>
              <span class="step-body">{step.body}</span>
            </span>
          </li>
        {/each}
      </ol>
    </aside>
  {/if}
</section>

<style>
  .hero {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 296px;
    align-items: start;
    gap: 56px;
    padding: 32px 0 8px;
  }

  .hero.compact {
    grid-template-columns: minmax(0, 1fr);
    gap: 0;
    padding: 18px 0 4px;
  }

  .hero.compact .hero-main {
    max-width: none;
  }

  .hero.compact .input-wrap {
    height: 42px;
  }

  .hero.compact .search-input {
    font-size: 13px;
  }

  .hero-main {
    min-width: 0;
    max-width: 620px;
  }

  .kicker {
    font-size: 10px;
    color: var(--faint);
    letter-spacing: 0.16em;
    text-transform: uppercase;
    margin-bottom: 14px;
  }

  h1 {
    margin: 0 0 26px;
    font-size: 46px;
    font-weight: 400;
    line-height: 1.06;
    letter-spacing: -0.015em;
    text-wrap: balance;
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
    height: 52px;
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
    width: 22px;
    flex-shrink: 0;
    color: var(--faint);
  }

  .search-input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    outline: none;
    font-size: 14px;
    color: var(--text);
    padding: 0 12px;
    height: 100%;
  }

  .search-input::placeholder {
    color: var(--faint);
  }

  .hint {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 7px;
    margin-top: 14px;
    font-size: 10.5px;
    color: var(--faint);
    letter-spacing: 0.04em;
  }

  .key {
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
    background: var(--bg-raise);
    color: var(--dim);
    font-size: 10px;
  }

  .hint-sep {
    width: 1px;
    height: 11px;
    background: var(--line-strong);
    margin: 0 3px;
  }

  .steps-panel {
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    padding: 16px 18px 18px;
  }

  .panel-label {
    font-size: 10px;
    color: var(--faint);
    letter-spacing: 0.16em;
    text-transform: uppercase;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--line);
  }

  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .step {
    display: grid;
    grid-template-columns: 18px 20px minmax(0, 1fr);
    align-items: start;
    gap: 10px;
    padding: 13px 0;
    border-bottom: 1px solid var(--line);
  }

  .step:last-child {
    border-bottom: none;
    padding-bottom: 4px;
  }

  .step-index {
    font-size: 10px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
    padding-top: 2px;
  }

  .step-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent);
  }

  .step-copy {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .step-title {
    font-family: var(--font-display);
    font-size: 13px;
    font-weight: 500;
    line-height: 1.3;
    color: var(--text);
  }

  .step-body {
    font-size: 11px;
    line-height: 1.45;
    color: var(--dim);
    text-wrap: pretty;
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

  @container player (max-width: 860px) {
    .hero {
      grid-template-columns: minmax(0, 1fr);
      gap: 28px;
    }

    .hero-main {
      max-width: none;
    }

    .steps {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 0 20px;
    }

    .step {
      grid-template-columns: 18px 20px minmax(0, 1fr);
    }
  }

  @container player (max-width: 620px) {
    .hero {
      padding: 20px 0 6px;
    }

    h1 {
      font-size: 32px;
      margin-bottom: 18px;
    }

    .input-wrap {
      height: 44px;
      padding: 0 10px;
    }

    .search-input {
      font-size: 13px;
    }

    .steps {
      grid-template-columns: minmax(0, 1fr);
    }

    .step {
      border-bottom: 1px solid var(--line);
    }

    .step:last-child {
      border-bottom: none;
      padding-bottom: 4px;
    }
  }

  @container player (max-width: 420px) {
    .hero {
      padding: 14px 0 4px;
    }

    h1 {
      font-size: 26px;
      margin-bottom: 14px;
    }

    .input-wrap {
      height: 40px;
    }

    .search-input {
      font-size: 12px;
    }

    .steps-panel {
      padding: 14px;
    }
  }
</style>