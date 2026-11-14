<script lang="ts">
  import { Link2, LoaderCircle, AlertCircle, X } from "lucide-svelte";
  import type { Collection, StreamImportResult } from "@lib/types";
  import { vynl } from "@lib/vynl";
  import { friendlyVaultError } from "@lib/errors";
  import { t } from "@lib/i18n";
  import { toasts } from "@lib/toast";
  import {
    getCurrentPlaylists,
    refreshPlaylists,
  } from "@state/playlists.svelte";
  import PlaylistPicker from "../vault/PlaylistPicker.svelte";

  let {
    onClose,
    onImported,
  }: {
    onClose: () => void;
    onImported: (result: StreamImportResult) => void;
  } = $props();

  type Phase = "link" | "destination" | "importing";
  let phase = $state<Phase>("link");
  let url = $state("");
  let resolving = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let collection = $state<Collection | null>(null);

  const playlists = $derived(getCurrentPlaylists());

  $effect(() => {
    if (phase !== "link") void refreshPlaylists();
  });

  function looksValid(raw: string): boolean {
    const v = raw.trim().toLowerCase();
    return (
      v.includes("open.spotify.com") ||
      v.includes("spotify:") ||
      v.includes("spotify.link") ||
      v.includes("spotify.app.link")
    );
  }

  async function resolveLink(): Promise<void> {
    const raw = url.trim();
    if (!raw || resolving) return;
    resolving = true;
    error = null;
    try {
      const c = await vynl.resolve(raw);
      if (c.kind !== "playlist") {
        error = t("moveToVynl.notPlaylist");
        return;
      }
      if (c.tracks.length === 0) {
        error = t("moveToVynl.empty");
        return;
      }
      collection = c;
      phase = "destination";
    } catch (e) {
      error = friendlyVaultError(e, t("moveToVynl.resolveFailed"));
    } finally {
      resolving = false;
    }
  }

  async function doImport(
    playlistId: string | null,
    playlistName: string | null,
  ): Promise<void> {
    if (!collection || busy) return;
    busy = true;
    error = null;
    phase = "importing";
    try {
      const result = await vynl.importStreamPlaylist(
        collection,
        playlistId,
        playlistName,
      );
      await refreshPlaylists();
      onImported(result);
    } catch (e) {
      const message = friendlyVaultError(e, t("moveToVynl.importFailed"));
      toasts.error(message);
      console.warn("stream playlist import failed:", e);
      busy = false;
      phase = "destination";
    }
  }

  function handleKey(e: KeyboardEvent): void {
    if (e.key !== "Escape") return;
    e.stopPropagation();
    if (phase === "importing") return;
    onClose();
  }

  const importLabel = $derived(
    collection
      ? t("moveToVynl.importing", { n: collection.tracks.length })
      : "",
  );
</script>

<svelte:window onkeydown={handleKey} />

{#if phase === "destination" && collection}
  <PlaylistPicker
    suggestedName={collection.title}
    trackCount={collection.tracks.length}
    playlists={playlists}
    onAddExisting={(id) => void doImport(id, null)}
    onCreate={(name) => void doImport(null, name)}
    onSkip={onClose}
  />
{:else}
  <div class="overlay" role="presentation">
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-label={t("moveToVynl.title")}
      tabindex="-1"
    >
      <header class="head">
        <span class="title">{t("moveToVynl.title")}</span>
        <button
          class="close-btn"
          onclick={onClose}
          disabled={phase === "importing"}
          aria-label={t("moveToVynl.close")}
        >
          <X size={15} stroke-width={1.75} />
        </button>
      </header>

      {#if phase === "importing"}
        <div class="importing">
          <LoaderCircle size={26} stroke-width={1.75} class="spin" />
          <p class="importing-label">{importLabel}</p>
          <p class="importing-note">{t("moveToVynl.importingNote")}</p>
        </div>
      {:else}
        <div class="body">
          <p class="description">{t("moveToVynl.description")}</p>
          <div class="input-wrap" class:warn={url.trim() !== "" && !looksValid(url)}>
            <Link2 size={15} stroke-width={1.75} />
            <input
              class="link-input"
              type="text"
              placeholder={t("moveToVynl.placeholder")}
              bind:value={url}
              spellcheck={false}
              autocomplete="off"
              autocapitalize="off"
              onkeydown={(e) => {
                if (e.key === "Enter") void resolveLink();
              }}
            />
          </div>
          {#if error}
            <div class="error" role="alert">
              <AlertCircle size={13} stroke-width={1.75} />
              <span>{error}</span>
            </div>
          {/if}
          <p class="note">{t("moveToVynl.note")}</p>
        </div>
        <footer class="foot">
          <button class="btn ghost" onclick={onClose}>
            {t("dialog.cancel")}
          </button>
          <button
            class="btn primary"
            disabled={resolving || url.trim().length === 0}
            onclick={() => void resolveLink()}
          >
            {#if resolving}
              <LoaderCircle size={14} stroke-width={2} class="spin" />
            {/if}
            <span>{t("moveToVynl.continue")}</span>
          </button>
        </footer>
      {/if}
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    z-index: 100;
  }

  .dialog {
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    width: 400px;
    max-width: calc(100vw - 48px);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    outline: none;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px;
    border-bottom: 1px solid var(--line);
  }

  .title {
    font-size: 14px;
    font-weight: 600;
  }

  .close-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    background: transparent;
    border: none;
    color: var(--dim);
    cursor: pointer;
  }

  .close-btn:hover:not(:disabled) {
    color: var(--text);
    background: var(--bg-raise);
  }

  .close-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .body {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .description {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--dim);
  }

  .input-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 40px;
    padding: 0 12px;
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    border: 1px solid var(--line-strong);
    color: var(--faint);
    transition: border-color 0.15s;
  }

  .input-wrap.warn {
    border-color: color-mix(in srgb, var(--red) 45%, transparent);
  }

  .link-input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    outline: none;
    font-size: 12px;
    color: var(--text);
  }

  .link-input::placeholder {
    color: var(--faint);
  }

  .error {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 9px 11px;
    border: 1px solid var(--danger-border);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--red) 6%, transparent);
    color: var(--red);
    font-size: 11px;
    line-height: 1.45;
  }

  .note {
    margin: 0;
    font-size: 10.5px;
    color: var(--faint);
    letter-spacing: 0.02em;
  }

  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 10px 16px 14px;
    border-top: 1px solid var(--line);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    height: 34px;
    padding: 0 14px;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.03em;
    cursor: pointer;
    white-space: nowrap;
    transition: color 0.15s, background 0.15s, border-color 0.15s;
  }

  .btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .btn.primary {
    color: var(--accent);
    background: var(--accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
  }

  .btn.primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
  }

  .btn.ghost {
    background: transparent;
    color: var(--dim);
    border: 1px solid var(--line-strong);
  }

  .btn.ghost:hover:not(:disabled) {
    color: var(--text);
    background: var(--bg-raise);
  }

  .importing {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 34px 24px 38px;
    color: var(--accent);
    text-align: center;
  }

  .importing-label {
    margin: 0;
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
  }

  .importing-note {
    margin: 0;
    max-width: 280px;
    font-size: 11px;
    line-height: 1.5;
    color: var(--faint);
  }

  .foot :global(.spin),
  .importing :global(.spin) {
    animation: spin 0.85s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
