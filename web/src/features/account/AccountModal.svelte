<script lang="ts">
  import { onMount } from "svelte";
  import { fade, fly } from "svelte/transition";
  import { cubicIn, cubicOut } from "svelte/easing";
  import { LoaderCircle, X, Dices } from "lucide-svelte";
  import { avatarProps, getAvatarCustom, saveAvatarCustom, randomAvatarSeed, avatarRevision } from "@lib/blobatar.svelte";
  import { Blobatar } from "@blobatar/svelte";
  import { t } from "@lib/i18n";
  import { vynl } from "@lib/vynl";
  import { toasts } from "@lib/toast";
  import { refreshPlaylists } from "@state/playlists.svelte";
  import { hydrateTaste } from "@state/taste.svelte";
  import { getCurrentSettings } from "@state/settings.svelte";
  import { resolveDisplayName } from "@lib/nickname.svelte";
  import {
    getSyncStatus,
    isCloudSyncing,
    isSyncStatusLoaded,
    refreshSyncStatus,
    runCloudSyncNow,
    setSyncStatus,
  } from "@state/sync.svelte";
  import Button from "@components/Button.svelte";
  import SettingRow from "@features/settings/SettingRow.svelte";
  import { focusTrap } from "@lib/actions/focus-trap";

  let { onclose }: { onclose: () => void } = $props();

  const status = $derived(getSyncStatus());
  const loaded = $derived(isSyncStatusLoaded());
  const syncing = $derived(isCloudSyncing());

  const syncHint = $derived.by(() => {
    if (!navigator.onLine) return t("account.syncOffline");
    if (status.lastError) return t("account.syncError", { error: status.lastError });
    if (status.pending) return t("account.syncPending");
    if (status.lastSyncedAt) {
      return t("account.lastSynced", {
        time: new Date(status.lastSyncedAt).toLocaleTimeString(),
      });
    }
    return t("account.syncUpToDate");
  });
  let custom = $state(getAvatarCustom());

  const avatarSeed = $derived(
    status.signedIn && status.email
      ? status.email
      : resolveDisplayName(getCurrentSettings().displayName),
  );
  const customSeed = $derived(custom.seed.trim());
  const effectiveSeed = $derived(customSeed || avatarSeed);
  const avatar = $derived.by(() => {
    void avatarRevision();
    return avatarProps(effectiveSeed, 96);
  });

  let mode = $state<"signin" | "signup">("signin");
  let email = $state("");
  let password = $state("");
  let busy = $state(false);
  let signingOut = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);

  const verb = $derived(
    busy
      ? mode === "signin"
        ? t("account.signingIn")
        : t("account.signingUp")
      : mode === "signin"
        ? t("account.signIn")
        : t("account.signUp"),
  );

  function rerollAvatar(): void {
    custom = { ...custom, seed: randomAvatarSeed() };
    saveAvatarCustom(custom);
  }

  const modeHint = $derived(
    mode === "signin" ? t("account.modeSync") : t("account.modeCreate"),
  );

  const canSubmit = $derived(email.trim().length > 0 && password.length > 0);

  onMount(() => {
    void refreshSyncStatus().catch(() => {});
  });

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape" && !event.defaultPrevented) {
      event.preventDefault();
      event.stopPropagation();
      onclose();
    }
  }

  function setMode(next: "signin" | "signup"): void {
    mode = next;
    error = null;
    notice = null;
  }

  async function submit(): Promise<void> {
    if (busy) return;
    const trimmedEmail = email.trim();
    if (!trimmedEmail || password.length === 0) {
      error = t("account.missingCredentials");
      notice = null;
      return;
    }
    error = null;
    notice = null;
    busy = true;
    try {
      const result =
        mode === "signin"
          ? await vynl.syncSignIn(trimmedEmail, password)
          : await vynl.syncSignUp(trimmedEmail, password);
      if (result.needsConfirmation) {
        notice = t("account.confirmationNeeded");
        password = "";
        return;
      }
      setSyncStatus(result.status);
      email = "";
      password = "";
      await refreshPlaylists();
      await hydrateTaste();
      toasts.success(
        mode === "signup"
          ? t("account.signUpSuccess")
          : t("account.welcomeBack"),
        4000,
      );
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function syncNow(): Promise<void> {
    if (syncing) return;
    try {
      await runCloudSyncNow();
      await refreshPlaylists();
      await hydrateTaste();
      toasts.success(t("account.syncDone"), 4000);
    } catch (e) {
      toasts.error(
        t("account.syncFailed", {
          error: e instanceof Error ? e.message : String(e),
        }),
        6000,
      );
    }
  }

  async function signOut(): Promise<void> {
    if (signingOut) return;
    signingOut = true;
    try {
      await vynl.syncSignOut();
      setSyncStatus({
        signedIn: false,
        email: null,
        userId: null,
        lastSyncedAt: null,
        lastError: null,
        pending: false,
      });
      notice = null;
      error = null;
      toasts.success(t("account.signedOut"), 4000);
    } catch (e) {
      toasts.error(
        t("account.signOutFailed", {
          error: e instanceof Error ? e.message : String(e),
        }),
        6000,
      );
    } finally {
      signingOut = false;
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" transition:fade={{ duration: 150 }} onclick={onclose}>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div
    class="panel"
    role="dialog"
    aria-modal="true"
    aria-label={t("account.title")}
    tabindex="-1"
    in:fly={{ y: 8, duration: 180, easing: cubicOut }}
    out:fly={{ y: 8, duration: 140, easing: cubicIn }}
    onclick={(event) => event.stopPropagation()}
    use:focusTrap
  >
    <div class="head">
      <span class="brand mono">VYNL</span>
      <button class="close" onclick={onclose} aria-label={t("common.close")}>
        <X size={12} stroke-width={1.6} />
      </button>
    </div>

    <form
      class="body"
      onsubmit={(event) => {
        event.preventDefault();
        void submit();
      }}
    >
      <div class="banner" class:connected={status.signedIn}>
        <Blobatar {...avatar} class="avatar" />
        <div class="banner-copy">
          <span class="caption mono">{t("common.account")}</span>
          <span class="banner-title">
            {#if status.signedIn}
              {status.email}
            {:else if loaded}
              {t("account.notSignedIn")}
            {:else}
              {t("account.checkingSession")}
            {/if}
          </span>
          <span class="banner-hint">
            {#if status.signedIn}
              {t("account.autoSyncHint")}
            {:else}
              {t("account.localHint")}
            {/if}
          </span>
        </div>
        <button
          type="button"
          class="reroll-btn"
          onclick={rerollAvatar}
          aria-label={t("account.avatarReroll")}
        >
          <Dices size={15} stroke-width={1.6} />
        </button>
      </div>

      {#if !loaded}
        <div class="loading mono"><LoaderCircle size={16} stroke-width={1.6} />{t("common.loading")}</div>
      {:else if status.signedIn}
        <div class="row-group">
          <SettingRow
            label={t("account.syncStatus")}
            hint={syncHint}
          >
            {#snippet control()}
              <Button
                variant="ghost"
                size="sm"
                disabled={syncing || !navigator.onLine}
                onclick={() => void syncNow()}
              >
                {#if syncing}
                  <span class="spin"><LoaderCircle size={13} stroke-width={1.6} /></span>
                  {t("account.syncing")}
                {:else}
                  {t("account.syncNow")}
                {/if}
              </Button>
            {/snippet}
          </SettingRow>
          <SettingRow
            label={t("account.signOut")}
            hint={t("account.signOutHint")}
          >
            {#snippet control()}
              <Button
                variant="danger"
                size="sm"
                disabled={signingOut}
                onclick={() => void signOut()}
              >
                {#if signingOut}
                  <span class="spin"><LoaderCircle size={13} stroke-width={1.6} /></span>
                  {t("account.signingOut")}
                {:else}
                  {t("account.signOut")}
                {/if}
              </Button>
            {/snippet}
          </SettingRow>
        </div>

        <div class="foot mono">{t("account.staysLocal")}</div>
      {:else}
        <div class="row-group">
          <SettingRow stacked label={t("account.email")}>
            {#snippet control()}
              <input
                class="dlg-input input-control"
                type="email"
                autocomplete="email"
                bind:value={email}
                placeholder={t("account.emailPlaceholder")}
                spellcheck={false}
                disabled={busy}
                required
              />
            {/snippet}
          </SettingRow>

          <SettingRow stacked label={t("account.password")}>
            {#snippet control()}
              <input
                class="dlg-input input-control"
                type="password"
                autocomplete={mode === "signin" ? "current-password" : "new-password"}
                bind:value={password}
                placeholder={t("account.passwordPlaceholder")}
                disabled={busy}
                required
              />
            {/snippet}
          </SettingRow>
        </div>

        {#if error}
          <div class="message error mono">{error}</div>
        {/if}
        {#if notice}
          <div class="message ok mono">{notice}</div>
        {/if}

        <div class="row-group actions">
          <SettingRow label={mode === "signin" ? t("account.signIn") : t("account.createAccount")} hint={modeHint}>
            {#snippet control()}
              <Button variant="primary" size="sm" disabled={busy || !canSubmit} onclick={() => void submit()}>
                {#if busy}
                  <span class="spin"><LoaderCircle size={13} stroke-width={1.6} /></span>
                  {verb}
                {:else}
                  {verb}
                {/if}
              </Button>
            {/snippet}
          </SettingRow>
        </div>

        <button
          type="button"
          class="switch-mode mono"
          disabled={busy}
          onclick={() => setMode(mode === "signin" ? "signup" : "signin")}
        >
          {mode === "signin"
            ? t("account.switchToSignUp")
            : t("account.switchToSignIn")}
        </button>

        <div class="foot mono">{t("account.staysLocal")}</div>
      {/if}
    </form>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 500;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.56);
    backdrop-filter: blur(5px);
  }

  .panel {
    width: min(500px, calc(100vw - 48px));
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
  }

  .head {
    display: flex;
    flex-shrink: 0;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 13px 16px;
    border-bottom: 1px solid var(--line);
  }

  .brand {
    color: var(--faint);
    font-size: 10px;
    letter-spacing: 0.2em;
    line-height: 1;
  }

  .close {
    display: flex;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    background: var(--bg-raise);
    color: var(--faint);
    transform: rotate(45deg);
    transition:
      color 0.15s,
      background 0.15s,
      border-color 0.15s;
  }

  .close :global(svg) {
    transform: rotate(-45deg);
  }

  .close:hover {
    color: var(--text);
    border-color: var(--line-strong);
  }

  .body {
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--line);
    background: var(--bg);
  }

  .banner.connected {
    background: color-mix(in srgb, var(--accent) 6%, var(--bg));
  }

  :global(.avatar) {
    width: 52px;
    height: 52px;
    flex-shrink: 0;
    border-radius: 50%;
    transform: scale(1.16);
  }

  .reroll-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    flex-shrink: 0;
    align-self: center;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--faint);
    transform: rotate(45deg);
    cursor: pointer;
    transition:
      color 0.15s,
      background 0.15s;
  }

  .reroll-btn :global(svg) {
    transform: rotate(-45deg);
  }

  .reroll-btn:hover {
    color: var(--text);
    background: var(--bg-raise);
  }


  .banner-copy {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 3px;
  }

  .caption {
    color: var(--faint);
    font-size: 9px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
  }

  .banner-title {
    color: var(--text);
    font-size: 14px;
    font-weight: 600;
    overflow-wrap: anywhere;
    line-height: 1.25;
  }

  .banner-hint {
    color: var(--faint);
    font-size: 10px;
    line-height: 1.45;
  }

  .row-group {
    display: flex;
    flex-direction: column;
    border-bottom: 1px solid var(--line);
  }

  .row-group.actions {
    border-top: 1px solid var(--line);
  }

  .input-control {
    width: 100%;
  }

  .message {
    margin: 12px 16px;
    padding: 10px 12px;
    border-left: 2px solid var(--red);
    background: color-mix(in srgb, var(--red) 8%, transparent);
    color: var(--red);
    font-size: 11px;
    line-height: 1.5;
  }

  .message.ok {
    border-color: var(--green);
    background: color-mix(in srgb, var(--green) 8%, transparent);
    color: var(--green);
  }

  .switch-mode {
    align-self: flex-start;
    margin: 12px 16px 0;
    color: var(--dim);
    font-size: 10px;
    letter-spacing: 0.04em;
    transition: color 0.15s;
  }

  .switch-mode:hover:not(:disabled) {
    color: var(--accent);
  }

  .switch-mode:disabled {
    cursor: default;
    opacity: 0.5;
  }

  .foot {
    padding: 12px 16px 14px;
    color: var(--faint);
    font-size: 9.5px;
    line-height: 1.6;
    letter-spacing: 0.02em;
  }

  .loading {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 28px 16px;
    color: var(--dim);
    font-size: 10px;
    letter-spacing: 0.08em;
  }

  .spin {
    display: flex;
  }

  .spin :global(svg),
  .loading :global(svg) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .spin :global(svg),
    .loading :global(svg) {
      animation: none;
    }
  }
</style>
