<script lang="ts">
  interface Props {
    balance: number;
    network: string;
    syncing?: boolean;
  }

  let { balance, network, syncing = false }: Props = $props();

  function formatSats(sats: number): string {
    return new Intl.NumberFormat().format(sats);
  }

  function formatBtc(sats: number): string {
    return (sats / 100_000_000).toFixed(8);
  }
</script>

<div class="balance-card">
  <div class="balance-glow"></div>
  <div class="balance-content">
    <div class="balance-label">Total Balance</div>
    <div class="balance-amount">
      <span class="sats">{formatSats(balance)}</span>
      <span class="unit">sats</span>
    </div>
    <div class="balance-btc">{formatBtc(balance)} BTC</div>
  </div>
  <div class="network-badge">
    <span class="pulse-dot" class:syncing></span>
    <span>{network}</span>
  </div>
</div>

<style>
  .balance-card {
    position: relative;
    margin: 0 var(--space-5, 20px);
    padding: 28px 24px;
    background: linear-gradient(135deg, var(--color-primary, #FBBF24) 0%, var(--color-primary-dark, #F59E0B) 100%);
    border-radius: var(--radius-3xl, 24px);
    overflow: hidden;
  }

  .balance-glow {
    position: absolute;
    top: -50%;
    right: -30%;
    width: 200px;
    height: 200px;
    background: radial-gradient(circle, rgba(255, 255, 255, 0.3) 0%, transparent 70%);
    pointer-events: none;
  }

  .balance-content {
    position: relative;
    text-align: center;
  }

  .balance-label {
    font-size: var(--text-md, 13px);
    font-weight: var(--font-medium, 500);
    color: rgba(0, 0, 0, 0.6);
    margin-bottom: 4px;
  }

  .balance-amount {
    display: flex;
    align-items: baseline;
    justify-content: center;
    gap: 8px;
    color: #000000;
  }

  .sats {
    font-size: 42px;
    font-weight: var(--font-bold, 700);
    font-family: var(--font-mono, 'SF Mono', 'Fira Code', monospace);
    letter-spacing: -2px;
  }

  .unit {
    font-size: var(--text-xl, 16px);
    font-weight: var(--font-semibold, 600);
    opacity: 0.8;
  }

  .balance-btc {
    font-size: var(--text-base, 14px);
    color: rgba(0, 0, 0, 0.5);
    font-family: var(--font-mono, 'SF Mono', 'Fira Code', monospace);
    margin-top: 4px;
  }

  .network-badge {
    position: absolute;
    top: 16px;
    right: 16px;
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(0, 0, 0, 0.15);
    padding: 6px 10px;
    border-radius: var(--radius-sm, 8px);
    font-size: var(--text-xs, 11px);
    font-weight: var(--font-semibold, 600);
    color: #000000;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .pulse-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #000000;
    animation: pulse 2s ease-in-out infinite;
  }

  .pulse-dot.syncing {
    animation: spin 1s linear infinite;
    background: transparent;
    border: 2px solid #000000;
    border-top-color: transparent;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.5; transform: scale(0.8); }
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
