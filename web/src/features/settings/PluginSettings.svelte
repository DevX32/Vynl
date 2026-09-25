<script lang="ts">
  import { t } from "@lib/i18n";
  import {
    getPluginConfig,
    getPluginSettingsSections,
    setPluginConfigValue,
  } from "@state/plugins.svelte";
  import type { PluginSettingField } from "@lib/plugins/types";

  const sections = $derived(getPluginSettingsSections());

  const timers = new Map<string, ReturnType<typeof setTimeout>>();

  function valueOf(
    pluginId: string,
    field: PluginSettingField,
  ): boolean | number | string {
    const raw = getPluginConfig(pluginId)[field.key];
    if (raw === undefined || raw === null) {
      return field.default ?? (field.kind === "boolean" ? false : "");
    }
    if (
      typeof raw === "boolean" ||
      typeof raw === "number" ||
      typeof raw === "string"
    ) {
      return raw;
    }
    return field.default ?? "";
  }

  function setNow(pluginId: string, key: string, value: unknown): void {
    void setPluginConfigValue(pluginId, key, value);
  }

  function setDebounced(pluginId: string, key: string, value: unknown): void {
    const handle = `${pluginId}:${key}`;
    const existing = timers.get(handle);
    if (existing) clearTimeout(existing);
    timers.set(
      handle,
      setTimeout(() => {
        timers.delete(handle);
        void setPluginConfigValue(pluginId, key, value);
      }, 300),
    );
  }
</script>

{#if sections.length > 0}
  <div class="panel">
    {#each sections as section (`${section.pluginId}:${section.id}`)}
      <div class="section">
        <div class="section-head">
          <span class="label mono">{section.title}</span>
          {#if section.pluginName}
            <span class="byline mono">
              {t("plugins.providedBy", { name: section.pluginName })}
            </span>
          {/if}
        </div>

        {#each section.fields as field (field.key)}
          {@const pluginId = section.pluginId ?? ""}
          {#if field.kind === "boolean"}
            <div class="field row-field">
              <div>
                <div class="field-label mono">{field.title}</div>
                {#if field.description}
                  <div class="hint mono">{field.description}</div>
                {/if}
              </div>
              <button
                class="switch"
                class:on={valueOf(pluginId, field) === true}
                role="switch"
                aria-checked={valueOf(pluginId, field) === true}
                aria-label={field.title}
                onclick={() =>
                  setNow(pluginId, field.key, !(valueOf(pluginId, field) === true))}
              >
                <span class="knob"></span>
              </button>
            </div>
          {:else if field.kind === "text"}
            <div class="field">
              <div class="field-label mono">{field.title}</div>
              {#if field.description}
                <div class="hint mono">{field.description}</div>
              {/if}
              <input
                class="text"
                value={String(valueOf(pluginId, field))}
                oninput={(e) =>
                  setDebounced(pluginId, field.key, e.currentTarget.value)}
                spellcheck={false}
              />
            </div>
          {:else if field.kind === "number"}
            <div class="field">
              <div class="field-label mono">{field.title}</div>
              {#if field.description}
                <div class="hint mono">{field.description}</div>
              {/if}
              <input
                class="text"
                type="number"
                value={String(valueOf(pluginId, field))}
                min={field.min}
                max={field.max}
                step={field.step}
                oninput={(e) => {
                  const raw = e.currentTarget.value;
                  if (raw === "") return;
                  setDebounced(pluginId, field.key, Number(raw));
                }}
              />
            </div>
          {:else if field.kind === "select"}
            <div class="field">
              <div class="field-label mono">{field.title}</div>
              {#if field.description}
                <div class="hint mono">{field.description}</div>
              {/if}
              <div class="chips">
                {#each field.options ?? [] as opt (opt.value)}
                  <button
                    class="chip mono"
                    class:selected={valueOf(pluginId, field) === opt.value}
                    onclick={() => setNow(pluginId, field.key, opt.value)}
                  >
                    {opt.label}
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        {/each}
      </div>
    {/each}
  </div>
{/if}

<style>
  .panel {
    display: flex;
    flex-direction: column;
  }

  .section {
    padding: 16px;
    border-top: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .section-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }

  .label {
    font-size: 11px;
    color: var(--dim);
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  .byline {
    font-size: 10.5px;
    color: var(--faint);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .row-field {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .field-label {
    font-size: 11px;
    color: var(--dim);
    letter-spacing: 0.08em;
  }

  .hint {
    font-size: 11px;
    color: var(--faint);
    line-height: 1.5;
  }

  .text {
    background: var(--bg-raise);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 10px 14px;
    outline: none;
    width: 100%;
    font-size: 12.5px;
    transition: border-color 0.15s;
  }

  .text:focus {
    border-color: var(--accent);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    font-size: 11px;
    color: var(--dim);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 4px 10px;
    transition:
      color 0.15s,
      background 0.15s;
  }

  .chip:hover {
    color: var(--text);
    background: var(--bg-raise);
  }

  .chip.selected {
    color: var(--accent);
    background: var(--accent-soft);
  }
</style>
