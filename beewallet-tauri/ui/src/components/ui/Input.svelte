<script lang="ts">
  interface Props {
    type?: 'text' | 'number' | 'password' | 'email';
    value?: string;
    placeholder?: string;
    label?: string;
    id?: string;
    disabled?: boolean;
    error?: string | null;
    monospace?: boolean;
  }

  let {
    type = 'text',
    value = $bindable(''),
    placeholder = '',
    label = '',
    id = '',
    disabled = false,
    error = null,
    monospace = false,
  }: Props = $props();
</script>

<div class="form-group" class:has-error={error}>
  {#if label}
    <label for={id}>{label}</label>
  {/if}
  <input
    {id}
    {type}
    class="input"
    class:monospace
    bind:value
    {placeholder}
    {disabled}
    autocomplete="off"
    autocapitalize="off"
    spellcheck="false"
  />
  {#if error}
    <span class="error-text">{error}</span>
  {/if}
</div>

<style>
  .form-group {
    display: flex;
    flex-direction: column;
    gap: var(--space-2, 8px);
  }

  label {
    font-size: var(--text-sm, 12px);
    font-weight: var(--font-semibold, 600);
    color: var(--color-text-tertiary, rgba(255, 255, 255, 0.5));
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .input {
    width: 100%;
    padding: 14px 16px;
    background: var(--color-surface-elevated, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-border, rgba(255, 255, 255, 0.08));
    border-radius: var(--radius-md, 12px);
    font-size: var(--text-lg, 15px);
    color: var(--color-text, #ffffff);
    transition: all var(--transition-base, 0.2s ease);
  }

  .input::placeholder {
    color: var(--color-text-muted, rgba(255, 255, 255, 0.3));
  }

  .input:focus {
    outline: none;
    border-color: var(--color-border-focus, rgba(251, 191, 36, 0.4));
    background: var(--color-surface-hover, rgba(255, 255, 255, 0.06));
  }

  .input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .input.monospace {
    font-family: var(--font-mono, 'SF Mono', 'Fira Code', monospace);
    font-size: var(--text-md, 13px);
  }

  .has-error .input {
    border-color: var(--color-error, #EF4444);
  }

  .error-text {
    font-size: var(--text-sm, 12px);
    color: var(--color-error, #EF4444);
  }
</style>
