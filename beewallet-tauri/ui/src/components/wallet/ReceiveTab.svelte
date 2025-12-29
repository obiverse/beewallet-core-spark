<script lang="ts">
  import { onMount } from 'svelte';
  import { Button } from '../ui';
  import { Wallet } from '../../lib/nine_s';

  /**
   * ReceiveTab - Spark-optimized receive flow (ported from Flutter ReceiveScreen)
   *
   * Philosophy: Show QR first, options second. Spark is instant magic.
   *
   * Payment methods:
   * - Instant: Spark address (default) - instant like Lightning
   * - On-chain: Bitcoin address for faucet/exchanges
   *
   * Default flow:
   * 1. Open → Show Spark address QR immediately (no decisions!)
   * 2. User can request specific amount → invoice
   * 3. Or switch to On-chain for faucet deposits
   */

  interface Props {
    address: string | null;  // Spark address
    onBack: () => void;
    onCreateInvoice: (amount: number, description?: string) => Promise<string>;
  }

  let { address, onBack, onCreateInvoice }: Props = $props();

  // Payment method: 'instant' (Spark) or 'onchain' (Bitcoin address for faucet)
  type PaymentMethod = 'instant' | 'onchain';
  let paymentMethod = $state<PaymentMethod>('instant');

  // Step: 'share' (show QR) or 'input' (request specific amount)
  type ReceiveStep = 'share' | 'input';
  let step = $state<ReceiveStep>('share');

  // Amount input
  let amount = $state('');
  let description = $state('');
  let loading = $state(false);
  let invoice = $state<string | null>(null);
  let copied = $state(false);

  // Bitcoin address for on-chain deposits
  let bitcoinAddress = $state<string | null>(null);
  let bitcoinFee = $state(0);
  let loadingBitcoin = $state(false);

  onMount(async () => {
    // Pre-fetch Bitcoin address for when user switches to on-chain
    await fetchBitcoinAddress();
  });

  async function fetchBitcoinAddress() {
    loadingBitcoin = true;
    try {
      const result = await Wallet.bitcoinAddress();
      bitcoinAddress = result.address;
      bitcoinFee = result.fee_sat;
    } catch (e) {
      console.error('Failed to get Bitcoin address:', e);
    } finally {
      loadingBitcoin = false;
    }
  }

  async function createInvoice() {
    if (!amount) return;

    loading = true;
    try {
      invoice = await onCreateInvoice(parseInt(amount), description || undefined);
      step = 'share';
    } catch (e) {
      console.error('Failed to create invoice:', e);
    } finally {
      loading = false;
    }
  }

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
    copied = true;
    setTimeout(() => copied = false, 2000);
  }

  function reset() {
    step = 'share';
    invoice = null;
    amount = '';
    description = '';
  }

  function switchToInput() {
    step = 'input';
  }

  function switchMethod(method: PaymentMethod) {
    paymentMethod = method;
    invoice = null;
  }

  function handleBack() {
    if (step === 'input') {
      reset();
    } else {
      onBack();
    }
  }
</script>

<div class="receive-tab">
  <div class="tab-header">
    <button class="back-btn" onclick={handleBack} aria-label="Back">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="19" y1="12" x2="5" y2="12"/>
        <polyline points="12 19 5 12 12 5"/>
      </svg>
    </button>
    <h2>{step === 'input' ? 'Request Amount' : 'Receive'}</h2>
  </div>

  <!-- Payment Method Tabs (from Flutter) -->
  {#if step === 'share'}
    <div class="method-tabs">
      <button
        class="method-tab"
        class:selected={paymentMethod === 'instant'}
        onclick={() => switchMethod('instant')}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
          <path d="M13 3L4 14h7v7l9-11h-7V3z"/>
        </svg>
        Instant
      </button>
      <button
        class="method-tab onchain-tab"
        class:selected={paymentMethod === 'onchain'}
        onclick={() => switchMethod('onchain')}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/>
          <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>
        </svg>
        On-chain
      </button>
    </div>
  {/if}

  <!-- Share Step (QR Display) -->
  {#if step === 'share'}
    <div class="share-view">
      {#if paymentMethod === 'instant'}
        <!-- Instant (Spark) Content -->
        {#if invoice}
          <!-- Show generated invoice -->
          <div class="badge instant">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M13 3L4 14h7v7l9-11h-7V3z"/>
            </svg>
            Spark Invoice
          </div>

          {#if amount}
            <div class="amount-display">
              <span class="amount-value">{parseInt(amount).toLocaleString()} sats</span>
            </div>
          {/if}

          <div class="qr-container">
            <div class="qr-box">
              <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="#888" stroke-width="1">
                <rect x="3" y="3" width="7" height="7"/>
                <rect x="14" y="3" width="7" height="7"/>
                <rect x="3" y="14" width="7" height="7"/>
                <rect x="14" y="14" width="3" height="3"/>
                <rect x="18" y="14" width="3" height="3"/>
                <rect x="14" y="18" width="3" height="3"/>
                <rect x="18" y="18" width="3" height="3"/>
              </svg>
              <span>Invoice QR</span>
            </div>
          </div>

          <p class="hint">Payment arrives in seconds</p>

          <div class="actions-row">
            <Button variant="primary" fullWidth onclick={() => copyToClipboard(invoice!)}>
              {#snippet icon()}
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  {#if copied}
                    <polyline points="20 6 9 17 4 12"/>
                  {:else}
                    <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
                  {/if}
                </svg>
              {/snippet}
              {copied ? 'Copied!' : 'Copy Invoice'}
            </Button>
          </div>

          <button class="text-btn" onclick={reset}>
            Back to Spark Address
          </button>
        {:else}
          <!-- Show Spark Address (default) -->
          <div class="badge instant">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M13 3L4 14h7v7l9-11h-7V3z"/>
            </svg>
            Spark Address
          </div>

          <div class="qr-container">
            <div class="qr-box">
              <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="#888" stroke-width="1">
                <rect x="3" y="3" width="7" height="7"/>
                <rect x="14" y="3" width="7" height="7"/>
                <rect x="3" y="14" width="7" height="7"/>
                <rect x="14" y="14" width="3" height="3"/>
                <rect x="18" y="14" width="3" height="3"/>
                <rect x="14" y="18" width="3" height="3"/>
                <rect x="18" y="18" width="3" height="3"/>
              </svg>
              <span>Spark QR</span>
            </div>
          </div>

          {#if address}
            <div class="address-display">
              <code>{address}</code>
              <button class="copy-btn-inline" onclick={() => copyToClipboard(address!)} aria-label="Copy">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  {#if copied}
                    <polyline points="20 6 9 17 4 12"/>
                  {:else}
                    <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
                  {/if}
                </svg>
              </button>
            </div>
          {/if}

          <p class="hint">Share this address to receive instant payments</p>

          <button class="outlined-btn" onclick={switchToInput}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
              <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
            </svg>
            Request specific amount
          </button>
        {/if}
      {:else}
        <!-- On-chain (Bitcoin) Content -->
        <div class="badge onchain">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <circle cx="12" cy="12" r="10"/>
          </svg>
          Bitcoin On-chain
        </div>

        <div class="qr-container">
          {#if loadingBitcoin}
            <div class="qr-box loading">
              <div class="spinner small"></div>
              <span>Loading...</span>
            </div>
          {:else if bitcoinAddress}
            <div class="qr-box">
              <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="#888" stroke-width="1">
                <rect x="3" y="3" width="7" height="7"/>
                <rect x="14" y="3" width="7" height="7"/>
                <rect x="3" y="14" width="7" height="7"/>
                <rect x="14" y="14" width="3" height="3"/>
                <rect x="18" y="14" width="3" height="3"/>
                <rect x="14" y="18" width="3" height="3"/>
                <rect x="18" y="18" width="3" height="3"/>
              </svg>
              <span>Bitcoin QR</span>
            </div>
          {:else}
            <div class="qr-box error">
              <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <line x1="12" y1="8" x2="12" y2="12"/>
                <line x1="12" y1="16" x2="12.01" y2="16"/>
              </svg>
              <span>Failed to load</span>
            </div>
          {/if}
        </div>

        {#if bitcoinAddress}
          <div class="address-display onchain">
            <div class="address-label">Bitcoin Address</div>
            <code>{bitcoinAddress}</code>
            <button class="copy-btn-inline" onclick={() => copyToClipboard(bitcoinAddress!)} aria-label="Copy">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                {#if copied}
                  <polyline points="20 6 9 17 4 12"/>
                {:else}
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
                {/if}
              </svg>
            </button>
          </div>

          {#if bitcoinFee > 0}
            <p class="fee-note">Deposit fee: {bitcoinFee.toLocaleString()} sats</p>
          {/if}
        {/if}

        <p class="hint">Confirmation takes ~10 minutes</p>

        <a href="https://app.lightspark.com/regtest-faucet" target="_blank" rel="noopener noreferrer" class="faucet-link">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
            <polyline points="15 3 21 3 21 9"/>
            <line x1="10" y1="14" x2="21" y2="3"/>
          </svg>
          Get regtest coins from faucet
        </a>
      {/if}
    </div>

  <!-- Input Step (Request Amount) -->
  {:else}
    <div class="input-view">
      <div class="form-group">
        <label for="recv-amount">Amount (sats)</label>
        <input
          id="recv-amount"
          type="number"
          class="input"
          bind:value={amount}
          placeholder="Amount to receive"
        />
      </div>

      <div class="form-group">
        <label for="recv-description">Description (optional)</label>
        <input
          id="recv-description"
          type="text"
          class="input"
          bind:value={description}
          placeholder="What's this payment for?"
        />
      </div>

      <Button
        variant="primary"
        size="lg"
        fullWidth
        loading={loading}
        disabled={!amount}
        onclick={createInvoice}
      >
        {#snippet icon()}
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M13 3L4 14h7v7l9-11h-7V3z"/>
          </svg>
        {/snippet}
        {loading ? 'Creating...' : 'Create Invoice'}
      </Button>
    </div>
  {/if}
</div>

<style>
  .receive-tab {
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
    margin-bottom: var(--space-5, 20px);
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

  /* Payment Method Tabs (ported from Flutter) */
  .method-tabs {
    display: flex;
    background: var(--color-surface, rgba(255, 255, 255, 0.03));
    border: 1px solid var(--color-border, rgba(255, 255, 255, 0.06));
    border-radius: var(--radius-md, 12px);
    padding: 4px;
    margin-bottom: var(--space-5, 20px);
  }

  .method-tab {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 10px 12px;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm, 8px);
    font-size: var(--text-md, 13px);
    font-weight: var(--font-medium, 500);
    color: var(--color-text-secondary, rgba(255, 255, 255, 0.5));
    cursor: pointer;
    transition: all var(--transition-base, 0.2s ease);
  }

  .method-tab:hover {
    color: var(--color-text-tertiary, rgba(255, 255, 255, 0.7));
  }

  .method-tab.selected {
    background: rgba(251, 191, 36, 0.15);
    color: #FBBF24;
  }

  .method-tab.onchain-tab.selected {
    background: rgba(247, 147, 26, 0.15);
    color: #F7931A;
  }

  /* Share View */
  .share-view {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    border-radius: 20px;
    font-size: var(--text-sm, 12px);
    font-weight: var(--font-semibold, 600);
    margin-bottom: var(--space-5, 20px);
  }

  .badge.instant {
    background: rgba(251, 191, 36, 0.15);
    color: #FBBF24;
  }

  .badge.onchain {
    background: rgba(247, 147, 26, 0.15);
    color: #F7931A;
  }

  .amount-display {
    margin-bottom: var(--space-4, 16px);
  }

  .amount-value {
    font-size: var(--text-4xl, 28px);
    font-weight: var(--font-bold, 700);
    color: var(--color-text, #ffffff);
  }

  .qr-container {
    margin-bottom: var(--space-4, 16px);
  }

  .qr-box {
    width: 220px;
    height: 220px;
    background: #ffffff;
    border-radius: var(--radius-xl, 16px);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3, 12px);
    color: #888888;
    font-size: var(--text-md, 13px);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.1);
  }

  .qr-box.loading {
    background: var(--color-surface, rgba(255, 255, 255, 0.03));
    color: var(--color-text-muted, rgba(255, 255, 255, 0.3));
  }

  .qr-box.error {
    background: var(--color-surface, rgba(255, 255, 255, 0.03));
    color: var(--color-error, #EF4444);
  }

  .spinner.small {
    width: 24px;
    height: 24px;
    border: 2px solid rgba(251, 191, 36, 0.2);
    border-top-color: #FBBF24;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .address-display {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    background: var(--color-surface, rgba(255, 255, 255, 0.03));
    border: 1px solid var(--color-border, rgba(255, 255, 255, 0.06));
    border-radius: var(--radius-md, 12px);
    margin-bottom: var(--space-3, 12px);
    max-width: 100%;
  }

  .address-display.onchain {
    flex-direction: column;
    align-items: stretch;
    text-align: left;
  }

  .address-label {
    font-size: var(--text-xs, 11px);
    font-weight: var(--font-semibold, 600);
    color: var(--color-text-muted, rgba(255, 255, 255, 0.4));
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 6px;
  }

  .address-display.onchain code {
    width: 100%;
    word-break: break-all;
    margin-bottom: 8px;
  }

  .address-display code {
    flex: 1;
    font-family: var(--font-mono, 'SF Mono', 'Fira Code', monospace);
    font-size: var(--text-sm, 12px);
    color: var(--color-text-tertiary, rgba(255, 255, 255, 0.6));
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .copy-btn-inline {
    padding: 6px;
    background: rgba(251, 191, 36, 0.12);
    border: none;
    border-radius: var(--radius-xs, 6px);
    color: #FBBF24;
    cursor: pointer;
    transition: all var(--transition-base, 0.2s ease);
    align-self: flex-end;
  }

  .copy-btn-inline:hover {
    background: rgba(251, 191, 36, 0.2);
  }

  .hint {
    font-size: var(--text-md, 13px);
    color: var(--color-text-muted, rgba(255, 255, 255, 0.4));
    margin-bottom: var(--space-5, 20px);
  }

  .fee-note {
    font-size: var(--text-sm, 12px);
    color: var(--color-text-muted, rgba(255, 255, 255, 0.4));
    margin-top: -8px;
    margin-bottom: var(--space-3, 12px);
  }

  .actions-row {
    width: 100%;
    max-width: 280px;
    margin-bottom: var(--space-4, 16px);
  }

  .outlined-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 12px 20px;
    background: transparent;
    border: 1px solid var(--color-border, rgba(255, 255, 255, 0.1));
    border-radius: var(--radius-md, 12px);
    color: var(--color-text-secondary, rgba(255, 255, 255, 0.6));
    font-size: var(--text-md, 13px);
    font-weight: var(--font-medium, 500);
    cursor: pointer;
    transition: all var(--transition-base, 0.2s ease);
  }

  .outlined-btn:hover {
    border-color: rgba(251, 191, 36, 0.3);
    color: #FBBF24;
  }

  .text-btn {
    background: none;
    border: none;
    color: var(--color-text-muted, rgba(255, 255, 255, 0.4));
    font-size: var(--text-md, 13px);
    cursor: pointer;
    transition: color var(--transition-base, 0.2s ease);
  }

  .text-btn:hover {
    color: var(--color-text-secondary, rgba(255, 255, 255, 0.6));
  }

  .faucet-link {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 12px 20px;
    background: rgba(247, 147, 26, 0.1);
    border: 1px solid rgba(247, 147, 26, 0.2);
    border-radius: var(--radius-md, 12px);
    color: #F7931A;
    font-size: var(--text-sm, 12px);
    font-weight: var(--font-medium, 500);
    text-decoration: none;
    transition: all var(--transition-base, 0.2s ease);
  }

  .faucet-link:hover {
    background: rgba(247, 147, 26, 0.15);
    border-color: rgba(247, 147, 26, 0.3);
  }

  /* Input View */
  .input-view {
    animation: fadeIn 0.3s ease-out;
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
</style>
