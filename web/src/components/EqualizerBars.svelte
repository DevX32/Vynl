<script lang="ts">
</script>

<span class="eq">
  <span class="bar"></span>
  <span class="bar"></span>
  <span class="bar"></span>
</span>

<style>
  .eq {
    display: inline-flex;
    align-items: flex-end;
    gap: 2px;
    height: 14px;
  }

  .bar {
    width: 3px;
    border-radius: 1px;
    background: var(--accent);
    animation: eq 0.8s ease-in-out infinite alternate;
  }

  .bar:nth-child(1) {
    height: 40%;
    animation-delay: 0s;
  }

  .bar:nth-child(2) {
    height: 70%;
    animation-delay: 0.2s;
  }

  .bar:nth-child(3) {
    height: 50%;
    animation-delay: 0.4s;
  }

  @keyframes eq {
    0% {
      height: 20%;
    }
    100% {
      height: 100%;
    }
  }
</style>
