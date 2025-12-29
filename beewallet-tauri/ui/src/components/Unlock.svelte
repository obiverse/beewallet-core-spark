<script lang="ts">
  import logoSvg from '../assets/logo.svg';
  import { Vault } from '../lib/nine_s';

  interface Props {
    onUnlock: () => void;
    onReset: () => void;
  }

  let { onUnlock, onReset }: Props = $props();

  let pin = $state('');
  let error = $state<string | null>(null);
  let loading = $state(false);
  let showResetConfirm = $state(false);
  let lockoutRemaining = $state(0);

  // Check vault status on mount
  $effect(() => {
    checkVaultStatus();
  });

  async function checkVaultStatus() {
    try {
      const status = await Vault.status();
      if (status) {
        lockoutRemaining = status.lockout_remaining;
      }
    } catch (e) {
      console.error('Failed to check vault status:', e);
    }
  }

  function handleDigit(digit: string) {
    if (pin.length < 6) {
      pin += digit;
      error = null;

      if (pin.length === 6) {
        setTimeout(() => attemptUnlock(), 150);
      }
    }
  }

  function handleDelete() {
    if (pin.length > 0) {
      pin = pin.slice(0, -1);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key >= '0' && e.key <= '9') {
      handleDigit(e.key);
    } else if (e.key === 'Backspace') {
      handleDelete();
    }
  }

  async function attemptUnlock() {
    loading = true;
    error = null;

    try {
      // Unlock vault with PIN
      await Vault.unlock(pin);

      // Auto-connect wallet after unlock
      await Vault.autoConnect('regtest');

      onUnlock();
    } catch (e: any) {
      const msg = e?.message || 'Unlock failed';

      // Check for rate limiting
      if (msg.includes('Rate limited')) {
        await checkVaultStatus();
        error = `Too many attempts. Try again in ${lockoutRemaining} seconds.`;
      } else if (msg.includes('Invalid passphrase')) {
        error = 'Incorrect PIN';
      } else {
        error = msg;
      }
      pin = '';
    } finally {
      loading = false;
    }
  }

  function confirmReset() {
    showResetConfirm = true;
  }

  function cancelReset() {
    showResetConfirm = false;
  }

  async function doReset() {
    try {
      // Reset the vault (destroys encrypted seed)
      await Vault.reset();
    } catch (e) {
      console.error('Failed to reset vault:', e);
    }
    showResetConfirm = false;
    onReset();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="unlock">
  {#if showResetConfirm}
    <div class="reset-confirm">
      <div class="warning-icon">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
          <line x1="12" y1="9" x2="12" y2="13"/>
          <line x1="12" y1="17" x2="12.01" y2="17"/>
        </svg>
      </div>
      <h2>Reset Wallet?</h2>
      <p class="warning-text">
        This will delete all wallet data. You will need your seed phrase to restore your wallet.
      </p>
      <div class="confirm-actions">
        <button class="btn-cancel" onclick={cancelReset}>Cancel</button>
        <button class="btn-danger" onclick={doReset}>Reset Wallet</button>
      </div>
    </div>
  {:else}
    <!-- Logo -->
    <div class="logo-container">
      <div class="logo-glow"></div>
      <img src={logoSvg} alt="BeeWallet" class="logo" />
    </div>

    <!-- Header -->
    <header>
      <h1>Welcome Back</h1>
      <p class="subtitle">Enter your PIN to unlock</p>
    </header>

    <!-- PIN Dots -->
    <div class="dots-container">
      <div class="dots" class:error={!!error} class:shake={!!error}>
        {#each Array(6) as _, i}
          <div class="dot" class:filled={i < pin.length}>
            {#if i < pin.length}
              <div class="dot-inner"></div>
            {/if}
          </div>
        {/each}
      </div>

      {#if error}
        <div class="error-message">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="8" x2="12" y2="12"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          {error}
        </div>
      {/if}
    </div>

    <!-- Keypad -->
    <div class="keypad">
      {#each [1, 2, 3, 4, 5, 6, 7, 8, 9] as digit}
        <button class="key" onclick={() => handleDigit(String(digit))} disabled={loading}>
          <span class="digit">{digit}</span>
        </button>
      {/each}
      <button class="key empty" disabled aria-hidden="true"></button>
      <button class="key" onclick={() => handleDigit('0')} disabled={loading}>
        <span class="digit">0</span>
      </button>
      <button class="key delete" onclick={handleDelete} disabled={loading} aria-label="Delete">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 4H8l-7 8 7 8h13a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2z"/>
          <line x1="18" y1="9" x2="12" y2="15"/>
          <line x1="12" y1="9" x2="18" y2="15"/>
        </svg>
      </button>
    </div>

    <!-- Reset Link -->
    <button class="reset-link" onclick={confirmReset}>
      Forgot PIN? Start over
    </button>
  {/if}
</div>

<style>
  .unlock {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 32px 24px;
    gap: 28px;
    animation: fadeIn 0.3s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* Logo */
  .logo-container {
    position: relative;
    width: 100px;
    height: 100px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .logo-glow {
    position: absolute;
    inset: -20px;
    background: radial-gradient(circle, rgba(251, 191, 36, 0.25) 0%, transparent 70%);
    border-radius: 50%;
    animation: glowPulse 3s ease-in-out infinite;
  }

  @keyframes glowPulse {
    0%, 100% { opacity: 0.5; transform: scale(1); }
    50% { opacity: 1; transform: scale(1.1); }
  }

  .logo {
    width: 64px;
    height: 64px;
    position: relative;
    z-index: 1;
    filter: drop-shadow(0 0 20px rgba(251, 191, 36, 0.4));
  }

  /* Header */
  header {
    text-align: center;
  }

  h1 {
    font-size: 28px;
    font-weight: 700;
    color: #ffffff;
    margin-bottom: 8px;
    letter-spacing: -0.5px;
  }

  .subtitle {
    font-size: 15px;
    color: rgba(255, 255, 255, 0.5);
  }

  /* PIN Dots */
  .dots-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }

  .dots {
    display: flex;
    gap: 16px;
    padding: 16px;
  }

  .dot {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 2px solid rgba(255, 255, 255, 0.2);
    background: transparent;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .dot.filled {
    border-color: #FBBF24;
    transform: scale(1.1);
  }

  .dot-inner {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #FBBF24;
    animation: dotPop 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: 0 0 12px rgba(251, 191, 36, 0.5);
  }

  @keyframes dotPop {
    0% { transform: scale(0); }
    50% { transform: scale(1.3); }
    100% { transform: scale(1); }
  }

  .dots.error .dot {
    border-color: #EF4444;
  }

  .dots.error .dot.filled {
    border-color: #EF4444;
  }

  .dots.error .dot-inner {
    background: #EF4444;
    box-shadow: 0 0 12px rgba(239, 68, 68, 0.5);
  }

  .dots.shake {
    animation: shake 0.4s ease-out;
  }

  @keyframes shake {
    0%, 100% { transform: translateX(0); }
    20% { transform: translateX(-8px); }
    40% { transform: translateX(8px); }
    60% { transform: translateX(-6px); }
    80% { transform: translateX(6px); }
  }

  /* Error Message */
  .error-message {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #EF4444;
    font-size: 14px;
    font-weight: 500;
    padding: 8px 16px;
    background: rgba(239, 68, 68, 0.1);
    border-radius: 8px;
  }

  /* Keypad */
  .keypad {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
    width: 100%;
    max-width: 280px;
  }

  .key {
    position: relative;
    aspect-ratio: 1.3;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 16px;
    cursor: pointer;
    transition: all 0.15s ease;
    user-select: none;
    -webkit-tap-highlight-color: transparent;
  }

  .digit {
    font-size: 28px;
    font-weight: 500;
    color: #ffffff;
  }

  .key:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(251, 191, 36, 0.2);
  }

  .key:active:not(:disabled) {
    transform: scale(0.95);
    background: rgba(251, 191, 36, 0.15);
    border-color: rgba(251, 191, 36, 0.4);
  }

  .key:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .key.empty {
    background: transparent;
    border-color: transparent;
    cursor: default;
  }

  .key.delete {
    color: rgba(255, 255, 255, 0.5);
  }

  .key.delete:hover:not(:disabled) {
    color: #EF4444;
    border-color: rgba(239, 68, 68, 0.3);
    background: rgba(239, 68, 68, 0.1);
  }

  /* Reset Link */
  .reset-link {
    background: none;
    border: none;
    color: rgba(255, 255, 255, 0.4);
    font-size: 14px;
    cursor: pointer;
    padding: 12px 16px;
    margin-top: 8px;
    transition: color 0.2s ease;
  }

  .reset-link:hover {
    color: rgba(255, 255, 255, 0.7);
  }

  /* Reset Confirm */
  .reset-confirm {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 20px;
    padding: 32px;
    text-align: center;
  }

  .warning-icon {
    width: 72px;
    height: 72px;
    background: rgba(239, 68, 68, 0.15);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #EF4444;
  }

  .warning-icon svg {
    width: 36px;
    height: 36px;
  }

  .reset-confirm h2 {
    font-size: 24px;
    font-weight: 700;
    color: #ffffff;
  }

  .warning-text {
    font-size: 15px;
    color: rgba(255, 255, 255, 0.6);
    line-height: 1.5;
    max-width: 280px;
  }

  .confirm-actions {
    display: flex;
    gap: 12px;
    width: 100%;
    margin-top: 8px;
  }

  .btn-cancel, .btn-danger {
    flex: 1;
    padding: 14px 20px;
    border-radius: 12px;
    font-size: 15px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .btn-cancel {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #ffffff;
  }

  .btn-cancel:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  .btn-danger {
    background: #EF4444;
    border: none;
    color: #ffffff;
  }

  .btn-danger:hover {
    background: #DC2626;
  }
</style>
