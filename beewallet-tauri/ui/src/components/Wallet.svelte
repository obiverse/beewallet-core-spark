<script lang="ts">
  import logoSvg from '../assets/logo.svg';
  import { wallet, refreshBalance, syncWallet } from '../lib/wallet.svelte';
  import * as api from '../lib/tauri';

  // Import sub-components
  import { BalanceCard, QuickActions, HomeTab, SendTab, ReceiveTab } from './wallet';
  import { IconButton } from './ui';

  // Tab state
  let activeTab = $state<'home' | 'send' | 'receive'>('home');

  // Address state (for receive tab)
  let address = $state<string | null>(null);

  $effect(() => {
    if (activeTab === 'receive' && !address) {
      loadAddress();
    }
  });

  async function loadAddress() {
    try {
      address = await api.getAddress();
    } catch (e) {
      console.error('Failed to get address:', e);
    }
  }

  async function handleSync() {
    await syncWallet();
    await refreshBalance();
  }

  async function handleSend(destination: string, amount?: number) {
    await api.sendPayment(destination, amount);
    await refreshBalance();
  }

  async function handleCreateInvoice(amount: number, description?: string): Promise<string> {
    const result = await api.createInvoice(amount, description);
    return result.invoice || result.bolt11;
  }
</script>

<div class="wallet">
  <!-- Header -->
  <header class="header">
    <div class="logo">
      <img src={logoSvg} alt="BeeWallet" class="logo-img" />
      <span class="logo-text">BeeWallet</span>
    </div>
    <IconButton ariaLabel="Sync wallet" onclick={handleSync} disabled={wallet.syncing}>
      <svg
        width="20"
        height="20"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        class:spinning={wallet.syncing}
      >
        <polyline points="23 4 23 10 17 10"/>
        <polyline points="1 20 1 14 7 14"/>
        <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
      </svg>
    </IconButton>
  </header>

  <!-- Balance Card -->
  <BalanceCard
    balance={wallet.balance}
    network="Regtest"
    syncing={wallet.syncing}
  />

  <!-- Quick Actions -->
  <QuickActions
    {activeTab}
    syncing={wallet.syncing}
    onSend={() => activeTab = 'send'}
    onReceive={() => activeTab = 'receive'}
    onSync={handleSync}
  />

  <!-- Tab Content -->
  <div class="tab-content">
    {#if activeTab === 'home'}
      <HomeTab />
    {:else if activeTab === 'send'}
      <SendTab
        onBack={() => activeTab = 'home'}
        onSend={handleSend}
      />
    {:else if activeTab === 'receive'}
      <ReceiveTab
        {address}
        onBack={() => activeTab = 'home'}
        onCreateInvoice={handleCreateInvoice}
      />
    {/if}
  </div>
</div>

<style>
  .wallet {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    background: linear-gradient(180deg, var(--color-bg, #0a0a0a) 0%, var(--color-bg-elevated, #121212) 100%);
    /* Center content on larger screens */
    max-width: var(--content-max-width-expanded, 800px);
    margin: 0 auto;
    width: 100%;
  }

  /* Header */
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4, 16px) var(--space-5, 20px);
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .logo-img {
    width: 32px;
    height: 32px;
  }

  .logo-text {
    font-size: var(--text-2xl, 18px);
    font-weight: var(--font-semibold, 600);
    color: var(--color-text, #ffffff);
  }

  .spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  /* Tab Content */
  .tab-content {
    flex: 1;
    padding: 0 var(--space-5, 20px) var(--space-6, 24px);
    overflow-y: auto;
  }

  /* Responsive: Expand padding on larger screens */
  @media (min-width: 600px) {
    .header {
      padding: var(--space-5, 20px) var(--space-6, 24px);
    }
    .tab-content {
      padding: 0 var(--space-6, 24px) var(--space-8, 32px);
    }
  }

  @media (min-width: 840px) {
    .header {
      padding: var(--space-6, 24px) var(--space-8, 32px);
    }
    .tab-content {
      padding: 0 var(--space-8, 32px) var(--space-10, 40px);
    }
  }
</style>
