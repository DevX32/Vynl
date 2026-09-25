<script lang="ts">
  import { t } from "@lib/i18n";
  import Button from "@components/Button.svelte";
  import Dialog from "@components/Dialog.svelte";
  import {
    Download,
    FileArchive,
    FolderOpen,
    RefreshCw,
    RotateCw,
    Trash2,
  } from "lucide-svelte";
  import {
    checkPluginUpdates,
    fetchStore,
    getInstalledPlugins,
    getPluginError,
    getPluginUpdates,
    getStoreError,
    getStoreLoading,
    getStorePlugins,
    installDevFolder,
    installFromStore,
    installZip,
    isPluginBusy,
    reloadDevPlugin,
    removePlugin,
    setPluginEnabled,
  } from "@state/plugins.svelte";
  import { getCurrentSettings, patchSettings } from "@state/settings.svelte";
  import type { PluginEntry, StorePlugin } from "@lib/plugins/types";

  let tab = $state<"installed" | "store">("installed");
  let query = $state("");
  let removeTarget = $state<PluginEntry | null>(null);

  const settings = $derived(getCurrentSettings());
  const installed = $derived(getInstalledPlugins());
  const store = $derived(getStorePlugins());
  const storeLoading = $derived(getStoreLoading());
  const storeError = $derived(getStoreError());
  const updateVersions = $derived(
    new Map(getPluginUpdates().map((u) => [u.entry.id, u.store.version])),
  );

  const q = $derived(query.trim().toLowerCase());

  function matches(haystack: string): boolean {
    return !q || haystack.toLowerCase().includes(q);
  }

  const filteredInstalled = $derived(
    installed.filter((p) =>
      matches(
        `${p.displayName ?? p.id} ${p.description ?? ""} ${p.author ?? ""} ${p.id}`,
      ),
    ),
  );

  const filteredStore = $derived(
    store.filter((p) =>
      matches(`${p.name} ${p.description} ${p.author} ${p.tags.join(" ")}`),
    ),
  );

  $effect(() => {
    void fetchStore();
  });

  function storeEntryFor(id: string): StorePlugin | undefined {
    return store.find((s) => s.id === id);
  }

  async function confirmRemove(): Promise<void> {
    const target = removeTarget;
    removeTarget = null;
    if (target) await removePlugin(target.id);
  }

  function categoryLabel(c: string): string {
    return t(`plugins.category.${c}`);
  }
</script>

<section class="plugins-page">
  <div class="body">
    <div class="opt-row">
      <div>
        <div class="opt-label mono">{t("settings.pluginsAutoUpdate")}</div>
        <div class="opt-hint mono">
          {t("settings.pluginsAutoUpdateHint")}
        </div>
      </div>
      <button
        class="switch"
        class:on={settings.pluginsAutoUpdate}
        onclick={() =>
          void patchSettings({ pluginsAutoUpdate: !settings.pluginsAutoUpdate })}
        role="switch"
        aria-checked={settings.pluginsAutoUpdate}
        aria-label={t("settings.pluginsAutoUpdateToggle")}
      >
        <span class="knob"></span>
      </button>
    </div>

    <div class="tabs" role="tablist">
      <button
        class="tab"
        class:active={tab === "installed"}
        role="tab"
        aria-selected={tab === "installed"}
        onclick={() => (tab = "installed")}
      >
        {t("plugins.tabInstalled")}
        <span class="tab-count mono">{installed.length}</span>
      </button>
      <button
        class="tab"
        class:active={tab === "store"}
        role="tab"
        aria-selected={tab === "store"}
        onclick={() => (tab = "store")}
      >
        {t("plugins.tabStore")}
      </button>
    </div>

    <div class="toolbar">
      <input
        class="text search"
        placeholder={t("plugins.searchPlaceholder")}
        value={query}
        oninput={(e) => (query = e.currentTarget.value)}
        spellcheck={false}
      />
      {#if tab === "installed"}
        <div class="toolbar-actions">
          <Button size="sm" onclick={() => void installDevFolder()}>
            <FolderOpen size={13} stroke-width={1.5} />
            {t("plugins.addFolder")}
          </Button>
          <Button size="sm" onclick={() => void installZip()}>
            <FileArchive size={13} stroke-width={1.5} />
            {t("plugins.installZip")}
          </Button>
        </div>
      {:else}
        <div class="toolbar-actions">
          <Button size="sm" onclick={() => void checkPluginUpdates(false)}>
            <RefreshCw size={13} stroke-width={1.5} />
            {t("plugins.checkUpdates")}
          </Button>
        </div>
      {/if}
    </div>

    {#if storeError}
      <div class="banner mono">
        {t("plugins.storeError", { error: storeError })}
      </div>
    {/if}

    {#if tab === "installed"}
      {#if filteredInstalled.length === 0}
        <div class="empty">
          <span class="empty-title">
            {installed.length === 0
              ? t("plugins.emptyInstalled")
              : t("plugins.searchNoResults")}
          </span>
          {#if installed.length === 0}
            <span class="empty-sub">{t("plugins.emptyInstalledSub")}</span>
          {/if}
        </div>
      {:else}
        <div class="list">
          {#each filteredInstalled as p (p.id)}
            {@const error = getPluginError(p.id)}
            {@const busy = isPluginBusy(p.id)}
            {@const updateTo = updateVersions.get(p.id)}
            <div class="row" class:has-error={!!error}>
              <div class="row-main">
                <div class="row-title-line">
                  <span class="row-name">{p.displayName ?? p.id}</span>
                  <span class="row-version mono"
                    >{t("plugins.version", { version: p.version })}</span
                  >
                  {#if p.installationMethod === "dev"}
                    <span class="badge mono dev">{t("plugins.devInstall")}</span>
                  {/if}
                  <span class="badge mono" class:on={p.enabled}>
                    {p.enabled ? t("plugins.enabled") : t("plugins.disabled")}
                  </span>
                  {#if updateTo}
                    <span class="badge mono update">
                      {t("plugins.version", { version: updateTo })}
                    </span>
                  {/if}
                </div>
                {#if p.description}
                  <span class="row-desc">{p.description}</span>
                {/if}
                <div class="row-meta mono">
                  {#if p.author}
                    <span>{t("plugins.author", { name: p.author })}</span>
                  {/if}
                  {#each p.categories as c (c)}
                    <span class="cat">{categoryLabel(c)}</span>
                  {/each}
                </div>
                {#if error}
                  <div class="row-error mono">
                    {t("plugins.loadError", { error })}
                  </div>
                {/if}
              </div>
              <div class="row-actions">
                {#if updateTo}
                  <Button
                    variant="primary"
                    size="sm"
                    disabled={busy}
                    onclick={() => {
                      const sp = storeEntryFor(p.id);
                      if (sp) void installFromStore(sp);
                    }}
                  >
                    <Download size={13} stroke-width={1.5} />
                    {busy ? t("plugins.updating") : t("plugins.update")}
                  </Button>
                {/if}
                {#if p.installationMethod === "dev"}
                  <Button
                    size="sm"
                    disabled={busy}
                    onclick={() => void reloadDevPlugin(p.id)}
                  >
                    <RotateCw size={13} stroke-width={1.5} />
                    {t("plugins.reload")}
                  </Button>
                {/if}
                <button
                  class="icon-btn"
                  title={t("plugins.remove")}
                  aria-label={t("plugins.remove")}
                  onclick={() => (removeTarget = p)}
                >
                  <Trash2 size={14} stroke-width={1.5} />
                </button>
                <button
                  class="switch"
                  class:on={p.enabled}
                  role="switch"
                  aria-checked={p.enabled}
                  aria-label={p.enabled
                    ? t("plugins.disable")
                    : t("plugins.enable")}
                  onclick={() => void setPluginEnabled(p.id, !p.enabled)}
                >
                  <span class="knob"></span>
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {:else}
      {#if storeLoading && store.length === 0}
        <div class="empty">
          <span class="empty-title">{t("plugins.loading")}</span>
        </div>
      {:else if filteredStore.length === 0}
        <div class="empty">
          <span class="empty-title">
            {store.length === 0
              ? t("plugins.emptyStore")
              : t("plugins.searchNoResults")}
          </span>
          {#if store.length === 0}
            <span class="empty-sub">{t("plugins.emptyStoreSub")}</span>
          {/if}
        </div>
      {:else}
        <div class="list">
          {#each filteredStore as sp (sp.id)}
            {@const busy = isPluginBusy(sp.id)}
            {@const entry = installed.find((p) => p.id === sp.id)}
            {@const hasUpdate = !!entry && updateVersions.has(sp.id)}
            <div class="row">
              <div class="row-main">
                <div class="row-title-line">
                  <span class="row-name">{sp.name}</span>
                  <span class="row-version mono"
                    >{t("plugins.version", { version: sp.version ?? "?" })}</span
                  >
                  {#if entry}
                    <span class="badge" class:on={entry.enabled}>
                      {entry.enabled
                        ? t("plugins.enabled")
                        : t("plugins.disabled")}
                    </span>
                  {/if}
                </div>
                <span class="row-desc">{sp.description}</span>
                <div class="row-meta mono">
                  <span>{t("plugins.author", { name: sp.author })}</span>
                  {#each sp.categories as c (c)}
                    <span class="cat">{categoryLabel(c)}</span>
                  {/each}
                </div>
              </div>
              <div class="row-actions">
                <Button
                  variant={entry ? "ghost" : "primary"}
                  size="sm"
                  disabled={busy || (!!entry && !hasUpdate)}
                  onclick={() => void installFromStore(sp)}
                >
                  <Download size={13} stroke-width={1.5} />
                  {#if busy}
                    {hasUpdate ? t("plugins.updating") : t("plugins.installing")}
                  {:else if hasUpdate}
                    {t("plugins.update")}
                  {:else if entry}
                    {t("plugins.installedBadge")}
                  {:else}
                    {t("plugins.install")}
                  {/if}
                </Button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</section>

<Dialog
  open={removeTarget !== null}
  title={t("plugins.removeTitle")}
  description={removeTarget
    ? t("plugins.removeBody", {
        name: removeTarget.displayName ?? removeTarget.id,
      })
    : ""}
  confirmLabel={t("plugins.removeConfirm")}
  danger
  onconfirm={() => void confirmRemove()}
  oncancel={() => (removeTarget = null)}
/>

<style>
  .plugins-page {
    display: flex;
    flex-direction: column;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .opt-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 14px;
    background: var(--bg-raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
  }

  .opt-label {
    font-size: 11px;
    color: var(--dim);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .opt-hint {
    font-size: 11px;
    color: var(--faint);
    line-height: 1.5;
    margin-top: 4px;
  }

  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--line);
    flex-shrink: 0;
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    font-family: var(--font-mono);
    font-size: 12px;
    letter-spacing: 0.06em;
    color: var(--faint);
    border-bottom: 1px solid transparent;
    margin-bottom: -1px;
    transition: color 0.15s, border-color 0.15s;
  }

  .tab:hover {
    color: var(--text);
  }

  .tab.active {
    color: var(--text);
    border-bottom-color: var(--accent);
  }

  .tab-count {
    font-size: 10px;
    color: var(--faint);
    background: var(--bg-raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    padding: 1px 6px;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .text {
    background: var(--bg-raise);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 10px 14px;
    height: 40px;
    outline: none;
    width: 100%;
    font-size: 12.5px;
    transition: border-color 0.15s;
  }

  .search {
    flex: 1 1 220px;
    min-width: 180px;
  }

  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .toolbar-actions :global(.btn) {
    height: 40px;
    padding: 0 14px;
    font-size: 12.5px;
  }

  .banner {
    font-size: 11px;
    color: var(--dim);
    background: var(--bg-raise);
    border: 1px solid var(--line);
    border-left: 2px solid var(--accent);
    border-radius: var(--radius-sm);
    padding: 8px 12px;
    line-height: 1.5;
    flex-shrink: 0;
  }

  .list {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    overflow: hidden;
    flex-shrink: 0;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 16px;
    background: var(--bg-raise);
    border-top: 1px solid var(--line);
  }

  .row:first-child {
    border-top: none;
  }

  .row.has-error {
    border-left: 2px solid var(--red);
  }

  .row-main {
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
  }

  .row-title-line {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }

  .row-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
  }

  .row-version {
    font-size: 10.5px;
    color: var(--faint);
  }

  .badge {
    font-size: 10px;
    letter-spacing: 0.06em;
    color: var(--faint);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 1px 7px;
    text-transform: uppercase;
  }

  .badge.on {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
    background: var(--accent-soft);
  }

  .badge.update {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
  }

  .badge.dev {
    color: var(--dim);
  }

  .row-desc {
    font-size: 12px;
    color: var(--dim);
    line-height: 1.5;
  }

  .row-meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    font-size: 10.5px;
    color: var(--faint);
  }

  .row-meta .cat {
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    padding: 1px 7px;
  }

  .row-error {
    font-size: 11px;
    color: var(--red);
    line-height: 1.5;
    word-break: break-word;
  }

  .row-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 48px 16px;
    text-align: center;
    border: 1px dashed var(--line-strong);
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }

  .empty-title {
    font-size: 13px;
    color: var(--dim);
  }

  .empty-sub {
    font-size: 12px;
    color: var(--faint);
  }
</style>
