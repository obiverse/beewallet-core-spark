<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    variant?: 'primary' | 'secondary' | 'ghost' | 'outlined';
    size?: 'sm' | 'md' | 'lg';
    disabled?: boolean;
    loading?: boolean;
    fullWidth?: boolean;
    type?: 'button' | 'submit';
    onclick?: () => void;
    children: Snippet;
    icon?: Snippet;
  }

  let {
    variant = 'primary',
    size = 'md',
    disabled = false,
    loading = false,
    fullWidth = false,
    type = 'button',
    onclick,
    children,
    icon,
  }: Props = $props();
</script>

<button
  {type}
  class="btn btn-{variant} btn-{size}"
  class:full-width={fullWidth}
  class:loading
  disabled={disabled || loading}
  {onclick}
>
  {#if loading}
    <svg class="spinner" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <circle cx="12" cy="12" r="10"/>
    </svg>
  {:else if icon}
    {@render icon()}
  {/if}
  <span class="btn-text">
    {@render children()}
  </span>
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    border: none;
    border-radius: var(--radius-lg, 14px);
    font-weight: 600;
    cursor: pointer;
    transition: all var(--transition-spring, 0.3s cubic-bezier(0.4, 0, 0.2, 1));
    position: relative;
    overflow: hidden;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .full-width {
    width: 100%;
  }

  /* Sizes */
  .btn-sm {
    height: 40px;
    padding: 0 16px;
    font-size: 13px;
  }

  .btn-md {
    height: 48px;
    padding: 0 20px;
    font-size: 14px;
  }

  .btn-lg {
    height: 54px;
    padding: 0 24px;
    font-size: 15px;
  }

  /* Primary */
  .btn-primary {
    background: linear-gradient(135deg, var(--color-primary, #FBBF24) 0%, var(--color-primary-dark, #F59E0B) 100%);
    color: #000000;
    box-shadow: var(--shadow-primary, 0 4px 16px rgba(251, 191, 36, 0.25));
  }

  .btn-primary:hover:not(:disabled) {
    transform: translateY(-2px);
    box-shadow: var(--shadow-primary-hover, 0 8px 24px rgba(251, 191, 36, 0.35));
  }

  .btn-primary:active:not(:disabled) {
    transform: translateY(-1px);
  }

  /* Secondary */
  .btn-secondary {
    background: var(--color-surface-elevated, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-border-strong, rgba(255, 255, 255, 0.1));
    color: var(--color-text-secondary, rgba(255, 255, 255, 0.7));
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--color-surface-hover, rgba(255, 255, 255, 0.06));
    border-color: rgba(255, 255, 255, 0.15);
    color: var(--color-text, #ffffff);
  }

  /* Ghost */
  .btn-ghost {
    background: transparent;
    color: var(--color-text-tertiary, rgba(255, 255, 255, 0.5));
  }

  .btn-ghost:hover:not(:disabled) {
    background: var(--color-surface, rgba(255, 255, 255, 0.03));
    color: var(--color-text-secondary, rgba(255, 255, 255, 0.7));
  }

  /* Outlined */
  .btn-outlined {
    background: transparent;
    border: 1px solid var(--color-border-strong, rgba(255, 255, 255, 0.1));
    color: var(--color-text-secondary, rgba(255, 255, 255, 0.7));
  }

  .btn-outlined:hover:not(:disabled) {
    background: var(--color-surface, rgba(255, 255, 255, 0.03));
    border-color: rgba(255, 255, 255, 0.15);
    color: var(--color-text, #ffffff);
  }

  /* Spinner */
  .spinner {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .btn-text {
    display: flex;
    align-items: center;
    gap: 8px;
  }
</style>
