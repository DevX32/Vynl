<script lang="ts">
  import { t } from "@lib/i18n";
  import { getPluginPages } from "@state/plugins.svelte";

  let { pageId }: { pageId: string } = $props();

  let container: HTMLDivElement | undefined = $state();

  const page = $derived(
    getPluginPages().find((p) => p.pluginId === pageId) ?? null,
  );

  $effect(() => {
    const el = container;
    const rec = page;
    if (!el || !rec) return;
    el.dataset.pluginPage = pageId;
    let fin: (() => void) | null = null;
    try {
      fin = rec.mount(el) ?? null;
    } catch (e) {
      console.warn(`[plugins] page "${pageId}" failed to mount:`, e);
    }
    return () => {
      if (fin !== null) {
        try {
          fin();
        } catch (e) {
          console.warn(`[plugins] page "${pageId}" cleanup failed:`, e);
        }
      }
      el.replaceChildren();
      delete el.dataset.pluginPage;
    };
  });
</script>

<div class="page-inner">
  {#if page}
    <div class="plugin-page" bind:this={container}></div>
  {:else}
    <p class="plugin-page-missing">{t("plugins.pageUnavailable")}</p>
  {/if}
</div>

<style>
  .plugin-page {
    display: contents;
  }

  .plugin-page-missing {
    color: var(--faint);
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 28px 8px;
  }
</style>
