<script lang="ts">
  import type { DragGhost } from "@lib/drag-list.svelte";

  let { ghost }: { ghost: DragGhost | null } = $props();
</script>

{#if ghost}
  <div class="drag-ghost" style:left="{ghost.x}px" style:top="{ghost.y}px">
    <span class="mono">{ghost.label}</span>
  </div>
{/if}
