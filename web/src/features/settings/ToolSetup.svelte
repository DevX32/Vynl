<script lang="ts">
  import { onMount } from "svelte";
  import type { ToolStatus } from "../../lib/types";
  import { vynl } from "../../lib/vynl";
  import { t } from "@lib/i18n";
  import {
    getCurrentTools,
    installTool,
    updateTool,
  } from "@state/settings.svelte";
  import Button from "../../components/Button.svelte";

  const tools = $derived(getCurrentTools());

  onMount(() => {
    void vynl.checkToolUpdates().catch(() => {});
  });

  function label(tool: ToolStatus): string {
    if (tool.state === "downloading")
      return t("tools.downloading", { n: Math.round(tool.progress ?? 0) });
    if (tool.state === "error")
      return tool.error
        ? t("tools.failed", { error: tool.error })
        : t("tools.failedUnknown");
    if (tool.installed) return tool.version ?? t("tools.installed");
    return t("tools.notInstalled");
  }

  const visibleTools = $derived(
    tools.filter(
      (tool) =>
        !tool.installed ||
        tool.state === "downloading" ||
        tool.state === "error" ||
        tool.updateAvailable,
    ),
  );
  const showBanner = $derived(
    visibleTools.some(
      (t) =>
        !t.installed ||
        t.state === "downloading" ||
        t.state === "error" ||
        t.updateAvailable,
    ),
  );
  const hasMissing = $derived(
    visibleTools.some((t) => !t.installed && t.state !== "downloading"),
  );
  const hasUpdates = $derived(visibleTools.some((t) => t.updateAvailable));
  const isInstalling = $derived(
    visibleTools.some((t) => t.state === "downloading"),
  );
</script>

{#if visibleTools.length > 0}
  <div class="setup" class:banner={showBanner}>
    {#if showBanner}
      <div class="head">
        <span class="title mono"
          >{hasMissing
            ? t("tools.missingTitle")
            : isInstalling
              ? t("tools.installingTitle")
              : t("tools.updatesTitle")}</span
        >
        <span class="hint mono">
          {hasMissing
            ? t("tools.missingHint")
            : isInstalling
              ? t("tools.installingHint")
              : hasUpdates
                ? t("tools.updatesHint")
                : ""}
        </span>
      </div>
    {/if}
    <div class="tools">
      {#each visibleTools as tool (tool.name)}
        <div class="tool" class:bad={tool.state === "error"}>
          <div class="tool-info">
            <span class="name mono">{tool.name}</span>
            <span
              class="ver mono"
              class:bad={(!tool.installed && tool.state !== "downloading") ||
                tool.state === "error"}
              >{label(tool)}</span
            >
          </div>
          {#if tool.state === "downloading"}
            <div class="bar">
              <div class="fill" style:width={`${tool.progress ?? 0}%`}></div>
            </div>
          {:else if tool.state === "error"}
            <Button
              variant="ghost"
              size="sm"
              onclick={() => void installTool(tool.name)}
              >{t("tools.retry")}</Button
            >
          {:else if !tool.installed}
            <Button
              variant="primary"
              size="sm"
              onclick={() => void installTool(tool.name)}
              >{t("tools.install")}</Button
            >
          {:else if tool.updateAvailable}
            <Button
              variant="ghost"
              size="sm"
              onclick={() => void updateTool(tool.name)}
              >{t("tools.update")}</Button
            >
          {/if}
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .setup {
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    background: var(--surface);
    padding: 16px 18px;
    margin-bottom: 18px;
    box-shadow: var(--shadow-sm);
  }

  .setup.banner {
    border-left: 2px solid var(--amber);
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: 12px;
    margin-bottom: 10px;
  }

  .title {
    font-size: 12px;
    letter-spacing: 0.1em;
    text-transform: capitalize;
  }

  .hint {
    color: var(--faint);
    font-size: 11px;
  }

  .tools {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .tool {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 14px;
    background: var(--bg-raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
  }

  .tool.bad {
    border-color: rgba(255, 107, 97, 0.4);
  }

  .tool-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .name {
    font-size: 12.5px;
  }

  .ver {
    font-size: 11px;
    color: var(--green);
  }

  .ver.bad {
    color: var(--amber);
  }

  .bar {
    flex: 0 0 100px;
    height: 3px;
    background: var(--line-strong);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--accent);
    border-radius: var(--radius-sm);
    transition: width 0.2s;
  }
</style>