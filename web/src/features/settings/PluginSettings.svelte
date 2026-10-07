<script lang="ts">
  import { t } from "@lib/i18n";
  import {
    getPluginConfig,
    getPluginSettingsSections,
    setPluginConfigValue,
  } from "@state/plugins.svelte";
  import type { PluginSettingField } from "@lib/plugins/types";
  import Button from "@components/Button.svelte";
  import Switch from "@components/Switch.svelte";
  import SettingRow from "./SettingRow.svelte";

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
  {#each sections as section (`${section.pluginId}:${section.id}`)}
    <div class="panel">
      <div class="panel-head">
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
          <SettingRow label={field.title} hint={field.description}>
            {#snippet control()}
              <Switch
                on={valueOf(pluginId, field) === true}
                label={field.title}
                onclick={() =>
                  setNow(
                    pluginId,
                    field.key,
                    !(valueOf(pluginId, field) === true),
                  )}
              />
            {/snippet}
          </SettingRow>
        {:else if field.kind === "text"}
            <SettingRow
            label={field.title}
            hint={field.description}
            stacked
          >
            {#snippet control()}
              <input
                class="text"
                value={String(valueOf(pluginId, field))}
                oninput={(e) =>
                  setDebounced(pluginId, field.key, e.currentTarget.value)}
                spellcheck={false}
              />
            {/snippet}
          </SettingRow>
        {:else if field.kind === "number"}
          <SettingRow
            label={field.title}
            hint={field.description}
            stacked
          >
            {#snippet control()}
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
            {/snippet}
          </SettingRow>
        {:else if field.kind === "select"}
          <SettingRow
            label={field.title}
            hint={field.description}
            stacked
          >
            {#snippet control()}
              {#each field.options ?? [] as opt (opt.value)}
                <Button
                  size="sm"
                  selected={valueOf(pluginId, field) === opt.value}
                  onclick={() => setNow(pluginId, field.key, opt.value)}
                >
                  {opt.label}
                </Button>
              {/each}
            {/snippet}
          </SettingRow>
        {/if}
      {/each}
    </div>
  {/each}
{/if}

<style>
  .panel {
    display: flex;
    flex-direction: column;
    margin-bottom: 18px;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
  }

  .panel-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    padding: 14px 16px;
    border-bottom: 1px solid var(--line);
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
</style>
