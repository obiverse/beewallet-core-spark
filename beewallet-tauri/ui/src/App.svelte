<script lang="ts">
  import { onMount } from 'svelte';
  import { wallet, checkConnection, refreshBalance } from './lib/wallet.svelte';
  import { reset } from './lib/onboarding.svelte';
  import { Vault, System } from './lib/nine_s';
  import Onboarding from './components/Onboarding.svelte';
  import Unlock from './components/Unlock.svelte';
  import Wallet from './components/Wallet.svelte';

  // Import shared styles
  import './styles/variables.css';
  import './styles/animations.css';

  // App state: 'loading' | 'onboarding' | 'unlock' | 'wallet' | 'migrate'
  type AppScreen = 'loading' | 'onboarding' | 'unlock' | 'wallet' | 'migrate';
  let screen = $state<AppScreen>('loading');
  let migrationRequired = $state(false);

  onMount(async () => {
    try {
      // Check vault status first
      const vaultStatus = await Vault.status();
      // Also check system status for wallet existence
      const systemStatus = await System.status();

      if (vaultStatus?.initialized) {
        // Vault exists - check if already unlocked
        if (vaultStatus.unlocked) {
          // Check if wallet is connected
          const connected = await checkConnection();
          if (connected) {
            wallet.connected = true;
            screen = 'wallet';
            refreshBalance();
          } else {
            // Vault unlocked but wallet not connected - auto-connect
            try {
              await Vault.autoConnect('regtest');
              wallet.connected = true;
              screen = 'wallet';
              refreshBalance();
            } catch (e) {
              // Connection failed, show unlock
              screen = 'unlock';
            }
          }
        } else {
          // Vault exists but locked - show unlock
          screen = 'unlock';
        }
      } else if (systemStatus?.wallet_exists) {
        // MIGRATION CASE: Wallet data exists but no vault
        // This means wallet was created before vault was implemented
        // User needs to restore from seed phrase to create vault
        console.warn('Wallet exists but vault not initialized - migration required');
        migrationRequired = true;
        screen = 'migrate';
      } else {
        // Fresh install - no vault, no wallet - show onboarding
        screen = 'onboarding';
      }
    } catch (e) {
      console.error('Init error:', e);
      screen = 'onboarding';
    }
  });

  function handleEnterWallet() {
    // Mark wallet as connected (onboarding already connected via api.connect)
    wallet.connected = true;
    screen = 'wallet';
    refreshBalance();
  }

  function handleUnlock() {
    wallet.connected = true;
    screen = 'wallet';
    refreshBalance();
  }

  let startInRestore = $state(false);

  function handleReset() {
    // Reset onboarding state and go back to onboarding
    reset();
    migrationRequired = false;
    startInRestore = false;
    screen = 'onboarding';
  }

  function handleMigrate() {
    // User acknowledged migration - go to restore flow
    startInRestore = true;
    screen = 'onboarding';
  }
</script>

<div class="app">
  {#if screen === 'loading'}
    <div class="loading">
      <div class="spinner"></div>
    </div>
  {:else if screen === 'wallet'}
    <Wallet />
  {:else if screen === 'unlock'}
    <Unlock onUnlock={handleUnlock} onReset={handleReset} />
  {:else if screen === 'migrate'}
    <div class="migrate-screen">
      <div class="migrate-icon">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
          <line x1="12" y1="9" x2="12" y2="13"/>
          <line x1="12" y1="17" x2="12.01" y2="17"/>
        </svg>
      </div>
      <h1>Wallet Upgrade Required</h1>
      <p class="migrate-text">
        Your wallet was created before PIN protection was added.
        To secure your funds, you need to <strong>restore your wallet</strong> using your 12-word seed phrase.
      </p>
      <p class="migrate-warning">
        Your seed phrase is the ONLY way to recover your funds.
        If you don't have it, your coins may be lost.
      </p>
      <button class="migrate-btn" onclick={handleMigrate}>
        I Have My Seed Phrase
      </button>
      <p class="migrate-hint">
        If you created a wallet recently, check your notes or password manager for the 12 words you wrote down during setup.
      </p>
    </div>
  {:else}
    <Onboarding onComplete={handleEnterWallet} {startInRestore} />
  {/if}
</div>

<style>
  :global(*) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(html, body) {
    height: 100%;
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }

  :global(body) {
    background: #0a0a0a;
    color: #ffffff;
  }

  :global(#app) {
    height: 100%;
  }

  .app {
    min-height: 100vh;
    width: 100%;
    background: linear-gradient(180deg, #0a0a0a 0%, #121212 100%);
  }

  :global(button) {
    font-family: inherit;
  }

  :global(input, textarea) {
    font-family: inherit;
  }

  :global(::selection) {
    background: rgba(251, 191, 36, 0.3);
  }

  /* Loading spinner */
  .loading {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid rgba(251, 191, 36, 0.2);
    border-top-color: #FBBF24;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Migration screen */
  .migrate-screen {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 32px 24px;
    text-align: center;
    animation: fadeIn 0.3s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .migrate-icon {
    width: 80px;
    height: 80px;
    background: rgba(251, 191, 36, 0.15);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #FBBF24;
    margin-bottom: 24px;
  }

  .migrate-icon svg {
    width: 40px;
    height: 40px;
  }

  .migrate-screen h1 {
    font-size: 24px;
    font-weight: 700;
    color: #ffffff;
    margin-bottom: 16px;
  }

  .migrate-text {
    font-size: 15px;
    color: rgba(255, 255, 255, 0.7);
    line-height: 1.6;
    max-width: 320px;
    margin-bottom: 16px;
  }

  .migrate-text strong {
    color: #FBBF24;
  }

  .migrate-warning {
    font-size: 14px;
    color: #EF4444;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 12px;
    padding: 12px 16px;
    max-width: 320px;
    margin-bottom: 24px;
  }

  .migrate-btn {
    width: 100%;
    max-width: 320px;
    padding: 16px 24px;
    background: #FBBF24;
    border: none;
    border-radius: 12px;
    font-size: 16px;
    font-weight: 600;
    color: #0a0a0a;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .migrate-btn:hover {
    background: #F59E0B;
    transform: translateY(-1px);
  }

  .migrate-hint {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.4);
    max-width: 280px;
    margin-top: 20px;
    line-height: 1.5;
  }
</style>
