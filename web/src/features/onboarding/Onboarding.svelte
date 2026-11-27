<script lang="ts">
  import { ArrowRight } from "lucide-svelte";
  import { getCurrentSettings } from "@state/settings.svelte";
  import { getNickname, rollNickname } from "@lib/nickname.svelte";
  import { completeOnboarding } from "@lib/onboarding.svelte";
  import { LOCALES, getLocale, setLocale, t } from "@lib/i18n";

  let { onComplete }: { onComplete: () => void } = $props();

  let step = $state<1 | 2>(1);
  let name = $state(getCurrentSettings().displayName);
  let placeholder = $state(getNickname());
  let saving = $state(false);

  function nextStep() {
    step = 2;
  }

  async function finish() {
    if (saving) return;
    saving = true;
    await completeOnboarding(name);
    onComplete();
  }

  function roll() {
    name = rollNickname();
    placeholder = name;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      if (step === 1) nextStep();
      else void finish();
    }
  }
</script>

<div class="onboarding" role="main" aria-label={t("onboarding.setName")}>
  <div class="card">
    <div class="kicker">GET STARTED</div>
    <div class="steps" aria-label="Onboarding steps">
      <span class="step-dot" class:active={step === 1}></span>
      <span class="step-dot" class:active={step === 2}></span>
    </div>

    {#if step === 1}
      <h1>Choose your<br />language</h1>
      <p class="sub">You can change this later in Settings.</p>

      <label class="field">
        <span class="field-label">{t("settings.language")}</span>
        <div class="lang-options">
          {#each LOCALES as locale (locale.id)}
            <button
              type="button"
              class="lang-opt"
              class:selected={getLocale() === locale.id}
              onclick={() => setLocale(locale.id)}
            >
              {locale.label}
            </button>
          {/each}
        </div>
      </label>

      <button class="btn cta" onclick={nextStep}>
        <span>Continue</span>
        <ArrowRight size={17} stroke-width={2} />
      </button>
    {:else}
      <h1>What should<br />we call you?</h1>
      <p class="sub">Your name across Vynl.</p>

      <label class="field">
        <span class="field-label">YOUR NAME</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          class="dlg-input name-input"
          bind:value={name}
          {placeholder}
          maxlength={32}
          autocomplete="off"
          autofocus
          onkeydown={onKeydown}
        />
      </label>
      <div class="row">
        <span class="hint">Leave blank for a random nickname</span>
        <button class="roll" type="button" onclick={roll}>Roll</button>
      </div>

      <div class="actions">
        <button class="btn secondary" type="button" onclick={() => (step = 1)} disabled={saving}>Back</button>
        <button class="btn cta" disabled={saving} onclick={finish}>
          <span>{saving ? "Saving..." : "Start listening"}</span>
          <ArrowRight size={17} stroke-width={2} />
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .onboarding {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: var(--bg);
    display: grid;
    place-items: center;
    overflow: auto;
  }

  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    width: min(460px, calc(100vw - 48px));
    padding: 24px;
  }

  .kicker {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.28em;
    color: var(--faint);
    margin-bottom: 24px;
  }

  h1 {
    font-family: var(--font-display);
    font-size: 44px;
    line-height: 1.04;
    font-weight: 650;
    letter-spacing: -0.035em;
    color: var(--text);
    margin: 0;
  }

  .sub {
    color: var(--dim);
    font-size: 13.5px;
    line-height: 1.55;
    margin: 14px 0 26px;
    max-width: 400px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
    text-align: left;
  }

  .field-label {
    font-family: var(--font-mono);
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0.14em;
    color: var(--faint);
  }

  .name-input {
    font-size: 15px;
    padding: 11px 12px;
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    width: 100%;
    margin-top: 8px;
  }

  .hint {
    color: var(--faint);
    font-size: 12px;
  }

  .roll {
    background: none;
    border: 0;
    color: var(--accent);
    font-weight: 650;
    font-size: 13px;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  .roll:hover {
    background: var(--accent-soft);
  }

  .cta {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    width: 100%;
    height: 52px;
    margin-top: 22px;
    font-size: 14px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 30%, transparent);
    box-shadow: none;
  }

  .cta:hover {
    background: color-mix(in srgb, var(--accent) 28%, transparent);
  }

  .steps {
    display: flex;
    gap: 8px;
    justify-content: center;
    margin: 0 0 20px;
  }

  .step-dot {
    width: 28px;
    height: 3px;
    border-radius: 999px;
    background: var(--line-strong);
    transition: background 0.2s ease, width 0.2s ease;
  }

  .step-dot.active {
    background: var(--accent);
    width: 40px;
  }

  .actions {
    display: flex;
    gap: 10px;
    width: 100%;
    margin-top: 22px;
  }

  .actions .cta {
    margin-top: 0;
    flex: 1;
  }

  .secondary {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 52px;
    padding: 0 18px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line-strong);
    background: transparent;
    color: var(--text);
    font: 600 13px inherit;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    cursor: pointer;
  }

  .secondary:hover:not(:disabled) {
    background: var(--bg-raise);
  }

  .secondary:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .lang-options {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .lang-opt {
    flex: 1;
    min-width: 0;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
    background: var(--bg-raise);
    color: var(--dim);
    font: 500 13px inherit;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s, background 0.15s;
  }

  .lang-opt:hover {
    color: var(--text);
    border-color: var(--line-strong);
  }

  .lang-opt.selected {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }
</style>
