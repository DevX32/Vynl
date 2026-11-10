<script lang="ts">
  import { t } from "@lib/i18n";
  import Button from "@components/Button.svelte";
  import { resetCatalogueTaste } from "@state/catalogue.svelte";
  import { getTasteSummary, resetTaste } from "@state/taste.svelte";
  import SettingRow from "./SettingRow.svelte";

  const summary = $derived(getTasteSummary());

  function reset(): void {
    resetTaste();
    resetCatalogueTaste();
  }
</script>

<div class="taste">
  <SettingRow
    label={t("settings.taste")}
    hint={summary.entries === 0
      ? t("settings.tasteEmpty")
      : t("settings.tasteHint", {
          liked: summary.liked,
          played: summary.played,
        })}
  >
    {#snippet control()}
      <Button
        size="sm"
        onclick={() => reset()}
        disabled={summary.entries === 0}
      >
        {t("settings.tasteReset")}
      </Button>
    {/snippet}
  </SettingRow>
</div>

<style>
  .taste {
    border-top: 1px solid var(--line);
  }
</style>
