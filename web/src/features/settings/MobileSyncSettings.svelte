<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "@lib/i18n";
  import Button from "@components/Button.svelte";
  import { openMobileSyncUi, refreshMobileSync } from "@state/mobile-sync.svelte";
  import SettingRow from "./SettingRow.svelte";

  onMount(() => {
    void refreshMobileSync();
  });
</script>

<SettingRow label={t("settings.mobileSync")} hint={t("settings.mobileSyncHint")}>
  {#snippet control()}
    <Button size="sm" onclick={openMobileSyncUi}>
      {t("plugins.open")}
    </Button>
  {/snippet}
</SettingRow>