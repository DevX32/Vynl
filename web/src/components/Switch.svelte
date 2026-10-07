<script lang="ts">
  let {
    on = false,
    disabled = false,
    label,
    onclick,
  }: {
    on?: boolean;
    disabled?: boolean;
    label: string;
    onclick: () => void;
  } = $props();
</script>

<button
  class="switch"
  class:on
  {disabled}
  role="switch"
  aria-checked={on}
  aria-label={label}
  onclick={onclick}
>
  <span class="knob"></span>
</button>

<style>
  .switch {
    position: relative;
    width: 36px;
    height: 20px;
    background: var(--line-strong);
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.2s;
    flex-shrink: 0;
    padding: 0;
  }

  .switch:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .switch.on {
    background: var(--accent-soft);
  }

  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 14px;
    height: 14px;
    background: var(--faint);
    border-radius: 3px;
    transition:
      transform 0.2s,
      background 0.2s;
  }

  .switch.on .knob {
    transform: translateX(16px);
    background: var(--accent);
  }
</style>