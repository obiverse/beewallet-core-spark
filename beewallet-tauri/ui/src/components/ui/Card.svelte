<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    variant?: 'default' | 'elevated' | 'bordered';
    padding?: 'none' | 'sm' | 'md' | 'lg';
    children: Snippet;
  }

  let {
    variant = 'default',
    padding = 'md',
    children,
  }: Props = $props();
</script>

<div class="card card-{variant} padding-{padding}">
  {@render children()}
</div>

<style>
  .card {
    border-radius: var(--radius-xl, 16px);
    transition: all var(--transition-base, 0.2s ease);
  }

  /* Variants */
  .card-default {
    background: var(--color-surface, rgba(255, 255, 255, 0.03));
    border: 1px solid var(--color-border, rgba(255, 255, 255, 0.06));
  }

  .card-elevated {
    background: var(--color-surface-elevated, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-border, rgba(255, 255, 255, 0.06));
    box-shadow: var(--shadow-md, 0 4px 16px rgba(0, 0, 0, 0.4));
  }

  .card-bordered {
    background: transparent;
    border: 1px solid var(--color-border-strong, rgba(255, 255, 255, 0.1));
  }

  /* Padding */
  .padding-none {
    padding: 0;
  }

  .padding-sm {
    padding: var(--space-3, 12px);
  }

  .padding-md {
    padding: var(--space-4, 16px);
  }

  .padding-lg {
    padding: var(--space-5, 20px);
  }
</style>
