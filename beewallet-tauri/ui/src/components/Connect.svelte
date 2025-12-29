<script lang="ts">
  import { wallet, connect, generateMnemonic } from '../lib/wallet.svelte';

  let mnemonic = $state('');
  let passphrase = $state('');
  let error = $state('');
  let showPassphrase = $state(false);

  async function handleGenerate() {
    try {
      mnemonic = await generateMnemonic();
      error = '';
    } catch (e: any) {
      error = e?.message || 'Failed to generate mnemonic';
    }
  }

  async function handleConnect() {
    if (!mnemonic.trim()) {
      error = 'Please enter a mnemonic phrase';
      return;
    }

    error = '';
    try {
      await connect(mnemonic.trim(), passphrase || undefined);
    } catch (e: any) {
      error = e?.message || 'Connection failed';
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !wallet.connecting) {
      handleConnect();
    }
  }
</script>

<div class="connect">
  <header>
    <div class="logo">
      <span class="bee">B</span>
    </div>
    <h1>BeeWallet Spark</h1>
    <p class="subtitle">Bitcoin L2 - Fast & Private</p>
  </header>

  <div class="card">
    <h2>Connect Wallet</h2>

    <div class="field">
      <label for="mnemonic">Seed Phrase</label>
      <textarea
        id="mnemonic"
        bind:value={mnemonic}
        placeholder="Enter your 12-word seed phrase..."
        rows="3"
        onkeydown={handleKeydown}
        disabled={wallet.connecting}
      ></textarea>
    </div>

    <div class="field">
      <label for="passphrase">
        Passphrase <span class="optional">(optional)</span>
      </label>
      <div class="password-field">
        {#if showPassphrase}
          <input
            id="passphrase"
            type="text"
            bind:value={passphrase}
            placeholder="BIP39 passphrase"
            disabled={wallet.connecting}
          />
        {:else}
          <input
            id="passphrase"
            type="password"
            bind:value={passphrase}
            placeholder="BIP39 passphrase"
            disabled={wallet.connecting}
          />
        {/if}
        <button
          type="button"
          class="toggle-visibility"
          onclick={() => showPassphrase = !showPassphrase}
        >
          {showPassphrase ? '🙈' : '👁️'}
        </button>
      </div>
    </div>

    {#if error}
      <div class="error">{error}</div>
    {/if}

    <div class="buttons">
      <button
        class="secondary"
        onclick={handleGenerate}
        disabled={wallet.connecting}
      >
        Generate New
      </button>
      <button
        class="primary"
        onclick={handleConnect}
        disabled={wallet.connecting || !mnemonic.trim()}
      >
        {wallet.connecting ? 'Connecting...' : 'Connect'}
      </button>
    </div>
  </div>

  <div class="info">
    <p>Using <strong>Regtest</strong> network for testing</p>
    <a href="https://app.lightspark.com/regtest-faucet" target="_blank" rel="noopener">
      Get test sats from faucet →
    </a>
  </div>
</div>

<style>
  .connect {
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding-top: 40px;
  }

  header {
    text-align: center;
  }

  .logo {
    width: 64px;
    height: 64px;
    background: linear-gradient(135deg, #f7931a 0%, #ffb347 100%);
    border-radius: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin: 0 auto 16px;
    box-shadow: 0 8px 32px rgba(247, 147, 26, 0.3);
  }

  .bee {
    font-size: 32px;
    font-weight: 800;
    color: #000;
  }

  h1 {
    font-size: 28px;
    font-weight: 700;
    margin-bottom: 4px;
  }

  .subtitle {
    color: #888;
    font-size: 14px;
  }

  .card {
    background: #1a1a1a;
    border: 1px solid #333;
    border-radius: 16px;
    padding: 24px;
  }

  .card h2 {
    font-size: 18px;
    font-weight: 600;
    margin-bottom: 20px;
  }

  .field {
    margin-bottom: 16px;
  }

  label {
    display: block;
    font-size: 13px;
    font-weight: 500;
    color: #aaa;
    margin-bottom: 8px;
  }

  .optional {
    color: #666;
    font-weight: 400;
  }

  textarea, input {
    width: 100%;
    padding: 14px;
    background: #0a0a0a;
    border: 1px solid #333;
    border-radius: 10px;
    color: #fff;
    font-size: 14px;
    font-family: inherit;
    resize: none;
    transition: border-color 0.2s;
  }

  textarea:focus, input:focus {
    outline: none;
    border-color: #f7931a;
  }

  textarea:disabled, input:disabled {
    opacity: 0.5;
  }

  .password-field {
    position: relative;
  }

  .password-field input {
    padding-right: 48px;
  }

  .toggle-visibility {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    font-size: 18px;
    cursor: pointer;
    padding: 4px;
  }

  .error {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid #ef4444;
    border-radius: 8px;
    padding: 12px;
    color: #ef4444;
    font-size: 13px;
    margin-bottom: 16px;
  }

  .buttons {
    display: flex;
    gap: 12px;
  }

  button {
    flex: 1;
    padding: 14px;
    border: none;
    border-radius: 10px;
    font-size: 15px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .primary {
    background: linear-gradient(135deg, #f7931a 0%, #ffb347 100%);
    color: #000;
  }

  .primary:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 4px 16px rgba(247, 147, 26, 0.4);
  }

  .secondary {
    background: #2a2a2a;
    color: #fff;
    border: 1px solid #444;
  }

  .secondary:hover:not(:disabled) {
    background: #333;
  }

  .info {
    text-align: center;
    color: #666;
    font-size: 13px;
  }

  .info a {
    color: #f7931a;
    text-decoration: none;
    display: block;
    margin-top: 8px;
  }

  .info a:hover {
    text-decoration: underline;
  }
</style>
