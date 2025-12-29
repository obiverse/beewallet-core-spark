<script lang="ts">
  import { isValidBip39Word } from '../../lib/onboarding.svelte';

  interface Props {
    words: string[];
    passphrase: string;
    showPassphrase: boolean;
    onConfirm: () => void;
    onBack: () => void;
  }

  let { words = $bindable(), passphrase = $bindable(), showPassphrase = $bindable(), onConfirm, onBack }: Props = $props();

  let focusedIndex = $state(-1);

  function getWordState(word: string): 'empty' | 'valid' | 'invalid' {
    if (!word.trim()) return 'empty';
    return isValidBip39Word(word.trim().toLowerCase()) ? 'valid' : 'invalid';
  }

  function handlePaste(e: ClipboardEvent) {
    const text = e.clipboardData?.getData('text') || '';
    const pastedWords = text.trim().split(/\s+/);

    if (pastedWords.length >= 12) {
      e.preventDefault();
      for (let i = 0; i < 12; i++) {
        words[i] = pastedWords[i] || '';
      }
    }
  }

  function handleKeydown(e: KeyboardEvent, index: number) {
    if (e.key === 'Enter' || (e.key === ' ' && words[index].trim())) {
      e.preventDefault();
      const nextIndex = index + 1;
      if (nextIndex < 12) {
        const nextInput = document.querySelector(`#word-input-${nextIndex}`) as HTMLInputElement;
        nextInput?.focus();
      }
    }
  }

  const validCount = $derived(words.filter(w => getWordState(w) === 'valid').length);
  const isComplete = $derived(validCount === 12);
</script>

<div class="restore">
  <!-- Header -->
  <header>
    <div class="icon-container">
      <div class="icon-bg">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="1 4 1 10 7 10"/>
          <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"/>
        </svg>
      </div>
    </div>
    <h2>Enter Recovery Phrase</h2>
    <p class="subtitle">Enter your 12-word seed phrase to restore your wallet.</p>
  </header>

  <!-- Progress Indicator -->
  <div class="progress-bar">
    <div class="progress-fill" style="width: {(validCount / 12) * 100}%"></div>
    <span class="progress-text" class:complete={isComplete}>
      {validCount}/12 words verified
    </span>
  </div>

  <!-- Word Grid -->
  <div class="grid">
    {#each words as word, i}
      {@const state = getWordState(word)}
      <div
        class="word-input"
        class:focused={focusedIndex === i}
        class:valid={state === 'valid'}
        class:invalid={state === 'invalid'}
      >
        <span class="index">{i + 1}</span>
        <input
          id="word-input-{i}"
          type="text"
          bind:value={words[i]}
          onfocus={() => focusedIndex = i}
          onblur={() => focusedIndex = -1}
          onpaste={handlePaste}
          onkeydown={(e) => handleKeydown(e, i)}
          placeholder=""
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
        />
        {#if state === 'valid'}
          <span class="check">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
          </span>
        {/if}
      </div>
    {/each}
  </div>

  <!-- Passphrase Section -->
  <div class="advanced-section">
    <button type="button" class="advanced-toggle" onclick={() => showPassphrase = !showPassphrase}>
      <div class="toggle-icon" class:open={showPassphrase}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="9 18 15 12 9 6"/>
        </svg>
      </div>
      <span>Advanced: BIP39 Passphrase</span>
    </button>

    {#if showPassphrase}
      <div class="passphrase-field">
        <input
          type="password"
          bind:value={passphrase}
          placeholder="Optional passphrase (25th word)"
        />
        <div class="passphrase-warning">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="8" x2="12" y2="12"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          <span>If you used a passphrase when creating this wallet, you must enter it exactly.</span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Actions -->
  <div class="actions">
    <button class="btn-secondary" onclick={onBack}>
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="19" y1="12" x2="5" y2="12"/>
        <polyline points="12 19 5 12 12 5"/>
      </svg>
      Back
    </button>
    <button class="btn-primary" onclick={onConfirm} disabled={!isComplete}>
      <span class="btn-text">Continue</span>
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="5" y1="12" x2="19" y2="12"/>
        <polyline points="12 5 19 12 12 19"/>
      </svg>
    </button>
  </div>
</div>

<style>
  .restore {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    padding: 32px 24px;
    gap: 20px;
    animation: fadeIn 0.4s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(12px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* Header */
  header {
    text-align: center;
    padding-top: 8px;
  }

  .icon-container {
    width: 72px;
    height: 72px;
    margin: 0 auto 16px;
  }

  .icon-bg {
    width: 100%;
    height: 100%;
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.15) 0%, rgba(59, 130, 246, 0.05) 100%);
    border: 1px solid rgba(59, 130, 246, 0.2);
    border-radius: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #3B82F6;
  }

  .icon-bg svg {
    width: 32px;
    height: 32px;
  }

  h2 {
    font-size: 26px;
    font-weight: 700;
    color: #ffffff;
    margin-bottom: 8px;
    letter-spacing: -0.5px;
  }

  .subtitle {
    font-size: 15px;
    color: rgba(255, 255, 255, 0.5);
  }

  /* Progress Bar */
  .progress-bar {
    position: relative;
    height: 36px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 18px;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .progress-fill {
    position: absolute;
    left: 0;
    top: 0;
    height: 100%;
    background: linear-gradient(90deg, rgba(16, 185, 129, 0.2), rgba(16, 185, 129, 0.3));
    border-radius: 18px;
    transition: width 0.3s ease;
  }

  .progress-text {
    position: relative;
    z-index: 1;
    font-size: 13px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.6);
    letter-spacing: 0.3px;
  }

  .progress-text.complete {
    color: #10B981;
  }

  /* Word Grid */
  .grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 8px;
  }

  @media (min-width: 400px) {
    .grid {
      grid-template-columns: repeat(3, 1fr);
    }
  }

  .word-input {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    background: rgba(255, 255, 255, 0.04);
    border: 2px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
    transition: all 0.2s ease;
  }

  .word-input.focused {
    border-color: rgba(251, 191, 36, 0.5);
    background: rgba(251, 191, 36, 0.05);
  }

  .word-input.valid {
    border-color: rgba(16, 185, 129, 0.4);
    background: rgba(16, 185, 129, 0.06);
  }

  .word-input.invalid {
    border-color: rgba(239, 68, 68, 0.4);
    background: rgba(239, 68, 68, 0.06);
  }

  .index {
    font-size: 10px;
    font-weight: 700;
    color: #FBBF24;
    background: rgba(251, 191, 36, 0.15);
    padding: 3px 6px;
    border-radius: 5px;
    min-width: 20px;
    text-align: center;
  }

  .word-input input {
    flex: 1;
    min-width: 0;
    padding: 6px 0;
    border: none;
    background: transparent;
    font-family: 'SF Mono', 'Fira Code', monospace;
    font-size: 13px;
    font-weight: 600;
    color: #ffffff;
    text-align: center;
  }

  .word-input input:focus {
    outline: none;
  }

  .word-input input::placeholder {
    color: rgba(255, 255, 255, 0.2);
  }

  .check {
    color: #10B981;
    display: flex;
    align-items: center;
  }

  /* Advanced Section */
  .advanced-section {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 14px;
    overflow: hidden;
  }

  .advanced-toggle {
    width: 100%;
    padding: 14px 16px;
    background: none;
    border: none;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 14px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.6);
    cursor: pointer;
    text-align: left;
    transition: all 0.2s ease;
  }

  .advanced-toggle:hover {
    background: rgba(255, 255, 255, 0.03);
    color: rgba(255, 255, 255, 0.8);
  }

  .toggle-icon {
    transition: transform 0.2s ease;
  }

  .toggle-icon.open {
    transform: rotate(90deg);
  }

  .passphrase-field {
    padding: 0 16px 16px;
  }

  .passphrase-field input {
    width: 100%;
    padding: 14px;
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 10px;
    font-size: 14px;
    color: #ffffff;
  }

  .passphrase-field input:focus {
    outline: none;
    border-color: rgba(251, 191, 36, 0.4);
  }

  .passphrase-field input::placeholder {
    color: rgba(255, 255, 255, 0.3);
  }

  .passphrase-warning {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin-top: 10px;
    font-size: 12px;
    color: #F59E0B;
    line-height: 1.4;
  }

  .passphrase-warning svg {
    flex-shrink: 0;
    margin-top: 1px;
  }

  /* Actions */
  .actions {
    display: flex;
    gap: 12px;
    margin-top: auto;
    padding-top: 8px;
  }

  .btn-secondary {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 16px 20px;
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 14px;
    font-size: 15px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.7);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .btn-secondary:hover {
    background: rgba(255, 255, 255, 0.05);
    border-color: rgba(255, 255, 255, 0.15);
    color: #ffffff;
  }

  .btn-primary {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    height: 54px;
    background: linear-gradient(135deg, #FBBF24 0%, #F59E0B 100%);
    border: none;
    border-radius: 14px;
    font-size: 15px;
    font-weight: 600;
    color: #000000;
    cursor: pointer;
    transition: all 0.3s ease;
    box-shadow:
      0 4px 16px rgba(251, 191, 36, 0.25),
      0 0 0 1px rgba(251, 191, 36, 0.1) inset;
  }

  .btn-primary:hover:not(:disabled) {
    transform: translateY(-2px);
    box-shadow:
      0 8px 24px rgba(251, 191, 36, 0.35),
      0 0 0 1px rgba(255, 255, 255, 0.2) inset;
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
