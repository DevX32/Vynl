<script lang="ts">
  import { t } from "@lib/i18n";
  import Button from "@components/Button.svelte";
  import Dialog from "@components/Dialog.svelte";
  import { resetCatalogueTaste } from "@state/catalogue.svelte";
  import { getTasteSummary, resetTaste } from "@state/taste.svelte";
  import SettingRow from "./SettingRow.svelte";

  const summary = $derived(getTasteSummary());
  let confirming = $state(false);

  function confirmReset(): void {
    resetTaste();
    resetCatalogueTaste();
    confirming = false;
  }
</script>

<div class="taste">
  <SettingRow
    label={t("settings.taste")}
    hint={summary.entries === 0
      ? t("settings.tasteEmpty")
      : t("settings.tasteHint")}
  >
    {#snippet control()}
      <Button
        size="sm"
        onclick={() => (confirming = true)}
        disabled={summary.entries === 0}
      >
        {t("common.reset")}
      </Button>
    {/snippet}
  </SettingRow>

  <Dialog
    open={confirming}
    title={t("settings.tasteResetTitle")}
    description={t("settings.tasteResetBody")}
    confirmLabel={t("common.reset")}
    danger
    onConfirm={confirmReset}
    onClose={() => (confirming = false)}
  />
</div>

<style>
  .taste {
    border-top: 1px solid var(--line);
  }
</style>
