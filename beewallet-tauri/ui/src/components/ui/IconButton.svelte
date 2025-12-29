<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    size?: 'sm' | 'md' | 'lg';
    variant?: 'ghost' | 'surface' | 'primary';
    disabled?: boolean;
    ariaLabel: string;
    onclick?: () => void;
    children: Snippet;
  }

  let {
    size = 'md',
    variant = 'surface',
    disabled = false,
    ariaLabel,
    onclick,
    children,
  }: Props = $props();
</script>

<button
  class="icon-btn icon-btn-{size} icon-btn-{variant}"
  {disabled}
  aria-label={ariaLabel}
  {onclick}
>
  {@render children()}
</button>

<style>
  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    cursor: pointer;
    transition: all var(--transition-base, 0.2s ease);
  }

  .icon-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Sizes */
  .icon-btn-sm {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm, 8px);
  }

  .icon-btn-md {
    width: 44px;
    height: 44px;
    border-radius: var(--radius-md, 12px);
  }

  .icon-btn-lg {
    width: 52px;
    height: 52px;
    border-radius: var(--radius-lg, 14px);
  }

  /* Variants */
  .icon-btn-ghost {
    background: transparent;
    color: var(--color-text-tertiary, rgba(255, 255, 255, 0.5));
  }

  .icon-btn-ghost:hover:not(:disabled) {
    background: var(--color-surface, rgba(255, 255, 255, 0.03));
    color: var(--color-text-secondary, rgba(255, 255, 255, 0.7));
  }

  .icon-btn-surface {
    background: var(--color-surface-elevated, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-border, rgba(255, 255, 255, 0.08));
    color: var(--color-text-tertiary, rgba(255, 255, 255, 0.6));
  }

  .icon-btn-surface:hover:not(:disabled) {
    border-color: rgba(251, 191, 36, 0.3);
    color: var(--color-primary, #FBBF24);
    background: rgba(251, 191, 36, 0.1);
  }

  .icon-btn-primary {
    background: rgba(251, 191, 36, 0.12);
    border: 1px solid rgba(251, 191, 36, 0.2);
    color: var(--color-primary, #FBBF24);
  }

  .icon-btn-primary:hover:not(:disabled) {
    background: rgba(251, 191, 36, 0.2);
  }
</style>
