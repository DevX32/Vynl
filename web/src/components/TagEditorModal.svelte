<script lang="ts">
  import { fade, fly } from "svelte/transition";
  import { cubicIn, cubicOut } from "svelte/easing";
  import { X } from "lucide-svelte";
  import { t } from "@lib/i18n";
  import { toasts } from "@lib/toast";
  import { friendlyError } from "@lib/errors";
  import Button from "@components/Button.svelte";
  import { focusTrap } from "@lib/actions/focus-trap";
  import { vynl } from "@lib/vynl";
  import type { TrackTags } from "@lib/types";
  import { closeTagEditor, getTagEditorPath } from "@state/tag-editor.svelte";

  const path = $derived(getTagEditorPath());

  let title = $state("");
  let artist = $state("");
  let album = $state("");
  let year = $state("");
  let trackNumber = $state("");
  let genre = $state("");
  let comment = $state("");
  let loading = $state(false);
  let saving = $state(false);

  $effect(() => {
    const p = path;
    if (!p) return;
    loading = true;
    void vynl
      .libraryReadTags(p)
      .then((tags: TrackTags) => {
        title = tags.title ?? "";
        artist = tags.artist ?? "";
        album = tags.album ?? "";
        year = tags.year != null ? String(tags.year) : "";
        trackNumber = tags.trackNumber != null ? String(tags.trackNumber) : "";
        genre = tags.genre ?? "";
        comment = tags.comment ?? "";
      })
      .catch((e) => {
        toasts.error(friendlyError(e, t("common.failed")));
      })
      .finally(() => {
        loading = false;
      });
  });

  function onKeydown(event: KeyboardEvent): void {
    if (event.key !== "Escape" || saving || event.defaultPrevented) return;
    event.preventDefault();
    event.stopPropagation();
    closeTagEditor();
  }

  async function save(): Promise<void> {
    const p = path;
    if (!p) return;
    saving = true;
    try {
      const tags: TrackTags = {
        title: title.trim() || null,
        artist: artist.trim() || null,
        album: album.trim() || null,
        year: year.trim() ? Number(year) : null,
        trackNumber: trackNumber.trim() ? Number(trackNumber) : null,
        genre: genre.trim() || null,
        comment: comment.trim() || null,
      };
      await vynl.libraryWriteTags(p, tags);
      toasts.success(t("library.tagsSaved"));
      closeTagEditor();
    } catch (e) {
      toasts.error(e instanceof Error ? e.message : String(e));
    } finally {
      saving = false;
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  class="overlay"
  transition:fade={{ duration: 150 }}
  onclick={closeTagEditor}
>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div
    class="panel"
    role="dialog"
    aria-modal="true"
    aria-label={t("library.editTags")}
    tabindex="-1"
    in:fly={{ y: 8, duration: 180, easing: cubicOut }}
    out:fly={{ y: 8, duration: 140, easing: cubicIn }}
    onclick={(e) => e.stopPropagation()}
    use:focusTrap
  >
    <div class="head">
      <span class="title display">{t("library.editTags")}</span>
      <button
        class="icon-btn close-btn"
        onclick={closeTagEditor}
        aria-label={t("common.close")}
      >
        <X size={12} stroke-width={1.6} />
      </button>
    </div>
    <div class="body">
      {#if loading}
        <div class="loading mono">{t("common.loading")}</div>
      {:else}
        <div class="field">
          <label class="label mono" for="tag-title">
            {t("common.titleLabel")}
          </label>
          <input id="tag-title" class="text" bind:value={title} />
        </div>
        <div class="field">
          <label class="label mono" for="tag-artist">
            {t("common.artistLabel")}
          </label>
          <input id="tag-artist" class="text" bind:value={artist} />
        </div>
        <div class="field">
          <label class="label mono" for="tag-album">{t("common.album")}</label>
          <input id="tag-album" class="text" bind:value={album} />
        </div>
        <div class="row">
          <div class="field">
            <label class="label mono" for="tag-year">{t("common.year")}</label>
            <input id="tag-year" class="text" bind:value={year} />
          </div>
          <div class="field">
            <label class="label mono" for="tag-track">
              {t("library.tagTrackNumber")}
            </label>
            <input id="tag-track" class="text" bind:value={trackNumber} />
          </div>
        </div>
        <div class="field">
          <label class="label mono" for="tag-genre">
            {t("library.tagGenre")}
          </label>
          <input id="tag-genre" class="text" bind:value={genre} />
        </div>
        <div class="field">
          <label class="label mono" for="tag-comment">
            {t("library.tagComment")}
          </label>
          <textarea id="tag-comment" class="text" bind:value={comment}
          ></textarea>
        </div>
      {/if}
    </div>
    <div class="foot">
      <Button size="sm" onclick={closeTagEditor}>{t("common.cancel")}</Button>
      <Button
        size="sm"
        onclick={() => void save()}
        disabled={loading || saving}
      >
        {saving ? t("common.saving") : t("common.save")}
      </Button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: var(--z-modal);
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--overlay-bg);
    backdrop-filter: blur(var(--overlay-blur));
  }
  .panel {
    width: min(520px, calc(100vw - 40px));
    max-height: 80vh;
    background: var(--bg-raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 14px 18px;
    border-bottom: 1px solid var(--line);
  }
  .title {
    font-size: 16px;
    font-weight: 600;
  }
  .close-btn:hover {
    background: var(--surface);
    border-color: var(--line-strong);
  }
  .body {
    padding: 18px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row {
    display: flex;
    gap: 12px;
  }
  .row .field {
    flex: 1;
  }
  .label {
    font-size: 11px;
    color: var(--dim);
    letter-spacing: 0.1em;
  }
  .text {
    background: var(--bg-raise);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    padding: 8px 10px;
    color: var(--text);
    font-size: 13px;
    width: 100%;
    resize: vertical;
  }
  .loading {
    color: var(--faint);
    font-size: 12px;
    text-align: center;
    padding: 24px 0;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 18px;
    border-top: 1px solid var(--line);
  }
</style>
