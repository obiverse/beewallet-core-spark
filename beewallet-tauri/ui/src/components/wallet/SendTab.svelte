<script lang="ts">
  import { Button } from '../ui';

  interface Props {
    onBack: () => void;
    onSend: (destination: string, amount?: number) => Promise<void>;
  }

  let { onBack, onSend }: Props = $props();

  let destination = $state('');
  let amount = $state('');
  let loading = $state(false);
  let error = $state<string | null>(null);
  let success = $state<string | null>(null);

  async function handleSend() {
    if (!destination.trim()) return;

    loading = true;
    error = null;
    success = null;

    try {
      const amountSat = amount ? parseInt(amount) : undefined;
      await onSend(destination.trim(), amountSat);
      success = 'Payment sent successfully!';
      destination = '';
      amount = '';
    } catch (e: any) {
      error = e?.message || String(e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="send-tab">
  <div class="tab-header">
    <button class="back-btn" onclick={onBack} aria-label="Back">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="19" y1="12" x2="5" y2="12"/>
        <polyline points="12 19 5 12 12 5"/>
      </svg>
    </button>
    <h2>Send Payment</h2>
  </div>

  {#if success}
    <div class="message success">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/>
        <polyline points="22 4 12 14.01 9 11.01"/>
      </svg>
      {success}
    </div>
  {/if}

  {#if error}
    <div class="message error">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/>
        <line x1="12" y1="8" x2="12" y2="12"/>
        <line x1="12" y1="16" x2="12.01" y2="16"/>
      </svg>
      {error}
    </div>
  {/if}

  <div class="form-group">
    <label for="destination">Destination</label>
    <textarea
      id="destination"
      class="input"
      bind:value={destination}
      placeholder="Spark address, BOLT11 invoice, or LNURL"
      rows="3"
    ></textarea>
  </div>

  <div class="form-group">
    <label for="amount">Amount (sats)</label>
    <input
      id="amount"
      type="number"
      class="input"
      bind:value={amount}
      placeholder="Optional for invoices with amount"
    />
  </div>

  <Button
    variant="primary"
    size="lg"
    fullWidth
    loading={loading}
    disabled={!destination.trim()}
    onclick={handleSend}
  >
    {#snippet icon()}
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="22" y1="2" x2="11" y2="13"/>
        <polygon points="22 2 15 22 11 13 2 9 22 2"/>
      </svg>
    {/snippet}
    {loading ? 'Sending...' : 'Send Payment'}
  </Button>
</div>

<style>
  .send-tab {
    animation: fadeIn 0.3s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .tab-header {
    display: flex;
    align-items: center;
    gap: var(--space-4, 16px);
    margin-bottom: var(--space-6, 24px);
  }

  .back-btn {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-md, 12px);
    border: 1px solid var(--color-border, rgba(255, 255, 255, 0.08));
    background: var(--color-surface-elevated, rgba(255, 255, 255, 0.04));
    color: var(--color-text-tertiary, rgba(255, 255, 255, 0.6));
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--transition-base, 0.2s ease);
  }

  .back-btn:hover {
    border-color: rgba(251, 191, 36, 0.3);
    color: var(--color-primary, #FBBF24);
  }

  .tab-header h2 {
    font-size: var(--text-3xl, 22px);
    font-weight: var(--font-semibold, 600);
    color: var(--color-text, #ffffff);
  }

  .message {
    display: flex;
    align-items: center;
    gap: var(--space-3, 12px);
    padding: 14px var(--space-4, 16px);
    border-radius: var(--radius-lg, 14px);
    margin-bottom: var(--space-4, 16px);
    font-size: var(--text-base, 14px);
    font-weight: var(--font-medium, 500);
  }

  .message.success {
    background: rgba(16, 185, 129, 0.12);
    border: 1px solid rgba(16, 185, 129, 0.2);
    color: var(--color-success, #10B981);
  }

  .message.error {
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid rgba(239, 68, 68, 0.2);
    color: var(--color-error, #EF4444);
  }

  .form-group {
    margin-bottom: var(--space-4, 16px);
  }

  .form-group label {
    display: block;
    font-size: var(--text-sm, 12px);
    font-weight: var(--font-semibold, 600);
    color: var(--color-text-tertiary, rgba(255, 255, 255, 0.5));
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: var(--space-2, 8px);
  }

  .input {
    width: 100%;
    padding: 14px var(--space-4, 16px);
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

  textarea.input {
    resize: none;
    font-family: var(--font-mono, 'SF Mono', 'Fira Code', monospace);
    font-size: var(--text-md, 13px);
  }
</style>
