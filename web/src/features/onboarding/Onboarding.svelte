<script lang="ts">
  import { ArrowRight } from "lucide-svelte";
  import { getCurrentSettings } from "@state/settings.svelte";
  import { getNickname, rollNickname } from "@lib/nickname.svelte";
  import { completeOnboarding } from "@lib/onboarding.svelte";

  let { onComplete }: { onComplete: () => void } = $props();

  let name = $state(getCurrentSettings().displayName);
  let placeholder = $state(getNickname());
  let saving = $state(false);

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
    if (e.key === "Enter") void finish();
  }
</script>

<div class="onboarding" role="main" aria-label="Set your name">
  <div class="card">
    <div class="kicker">GET STARTED</div>
    <h1>What should<br />we call you?</h1>
    <p class="sub">Your name across Vynl.</p>

    <label class="field">
      <span class="field-label">YOUR NAME</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="dlg-input name-input"
        bind:value={name}
        placeholder={placeholder}
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

    <button class="btn cta" disabled={saving} onclick={finish}>
      <span>{saving ? "Saving…" : "Start listening"}</span>
      <ArrowRight size={17} stroke-width={2} />
    </button>
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
</style>
