<script lang="ts">
  interface Props {
    current: number;
    total: number;
  }

  let { current, total }: Props = $props();
</script>

<div class="step-indicator">
  <div class="bars">
    {#each Array(total) as _, i}
      <div class="bar" class:filled={i < current}></div>
    {/each}
  </div>
  <span class="label">Step {current} of {total}</span>
</div>

<style>
  .step-indicator {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-sm) 0 var(--spacing-lg);
  }

  .bars {
    flex: 1;
    display: flex;
    gap: var(--spacing-xs);
  }

  .bar {
    flex: 1;
    height: 4px;
    background: var(--color-text-tertiary);
    border-radius: 2px;
    opacity: 0.3;
    transition: all 0.3s ease;
  }

  .bar.filled {
    background: var(--color-honey);
    opacity: 1;
  }

  .label {
    font-size: 12px;
    color: var(--color-text-secondary);
    white-space: nowrap;
  }
</style>
