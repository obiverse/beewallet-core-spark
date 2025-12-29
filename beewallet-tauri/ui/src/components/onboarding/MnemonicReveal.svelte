<script lang="ts">
  interface Props {
    mnemonic: string[];
    onConfirm: () => void;
    onBack: () => void;
  }

  let { mnemonic, onConfirm, onBack }: Props = $props();

  let copied = $state(false);
  let revealed = $state(false);

  function copyPhrase() {
    navigator.clipboard.writeText(mnemonic.join(' '));
    copied = true;
    setTimeout(() => copied = false, 2000);
  }
</script>

<div class="reveal">
  <!-- Header -->
  <header>
    <div class="icon-container">
      <div class="icon-glow"></div>
      <div class="icon-bg">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M15.75 5.25a3 3 0 0 1 3 3m3 0a6 6 0 0 1-7.029 5.912c-.563-.097-1.159.026-1.563.43L10.5 17.25H8.25v2.25H6v2.25H2.25v-2.818c0-.597.237-1.17.659-1.591l6.499-6.499c.404-.404.527-1 .43-1.563A6 6 0 1 1 21.75 8.25Z"/>
        </svg>
      </div>
    </div>
    <h2>Your Recovery Phrase</h2>
    <p class="subtitle">These 12 words are the key to your wallet. Write them down and store them safely.</p>
  </header>

  <!-- Mnemonic Grid -->
  <div class="mnemonic-container" class:revealed>
    {#if !revealed}
      <div class="blur-overlay" onclick={() => revealed = true}>
        <div class="eye-icon">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
            <circle cx="12" cy="12" r="3"/>
          </svg>
        </div>
        <span>Tap to reveal</span>
      </div>
    {/if}

    <div class="grid">
      {#each mnemonic as word, i}
        <div class="word-chip">
          <span class="index">{i + 1}</span>
          <span class="word">{word}</span>
        </div>
      {/each}
    </div>
  </div>

  <!-- Copy Button -->
  <button class="copy-btn" onclick={copyPhrase} disabled={!revealed}>
    {#if copied}
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="20 6 9 17 4 12"/>
      </svg>
      Copied to clipboard!
    {:else}
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
        <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
      </svg>
      Copy to clipboard
    {/if}
  </button>

  <!-- Warning Card -->
  <div class="warning-card">
    <div class="warning-icon">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 9v4m0 4h.01M5.07 19h13.86c1.54 0 2.5-1.67 1.73-3L13.73 4c-.77-1.33-2.69-1.33-3.46 0L3.34 16c-.77 1.33.19 3 1.73 3z"/>
      </svg>
    </div>
    <div class="warning-content">
      <strong>Keep this phrase secret</strong>
      <p>Anyone with these words can access your funds. Never share them with anyone.</p>
    </div>
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
    <button class="btn-primary" onclick={onConfirm} disabled={!revealed}>
      <span class="btn-text">I've Written It Down</span>
      <div class="btn-shine"></div>
    </button>
  </div>
</div>

<style>
  .reveal {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    padding: 32px 24px;
    gap: 24px;
    animation: fadeIn 0.4s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(12px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* Header */
  header {
    text-align: center;
    padding-top: 16px;
  }

  .icon-container {
    position: relative;
    width: 80px;
    height: 80px;
    margin: 0 auto 20px;
  }

  .icon-glow {
    position: absolute;
    inset: 0;
    background: radial-gradient(circle, rgba(251, 191, 36, 0.3) 0%, transparent 70%);
    border-radius: 50%;
    animation: iconPulse 2s ease-in-out infinite;
  }

  @keyframes iconPulse {
    0%, 100% { transform: scale(1); opacity: 0.5; }
    50% { transform: scale(1.2); opacity: 0.8; }
  }

  .icon-bg {
    position: relative;
    width: 100%;
    height: 100%;
    background: linear-gradient(135deg, rgba(251, 191, 36, 0.15) 0%, rgba(251, 191, 36, 0.05) 100%);
    border: 1px solid rgba(251, 191, 36, 0.2);
    border-radius: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .icon-bg svg {
    width: 36px;
    height: 36px;
    color: #FBBF24;
  }

  h2 {
    font-size: 28px;
    font-weight: 700;
    color: #ffffff;
    margin-bottom: 8px;
    letter-spacing: -0.5px;
  }

  .subtitle {
    font-size: 15px;
    color: rgba(255, 255, 255, 0.6);
    line-height: 1.5;
    max-width: 320px;
    margin: 0 auto;
  }

  /* Mnemonic Container */
  .mnemonic-container {
    position: relative;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 20px;
    padding: 16px;
    overflow: hidden;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 8px;
    transition: filter 0.3s ease;
  }

  .mnemonic-container:not(.revealed) .grid {
    filter: blur(8px);
    user-select: none;
  }

  @media (min-width: 400px) {
    .grid {
      grid-template-columns: repeat(3, 1fr);
    }
  }

  .blur-overlay {
    position: absolute;
    inset: 0;
    background: rgba(10, 10, 10, 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    cursor: pointer;
    z-index: 10;
    transition: all 0.3s ease;
  }

  .blur-overlay:hover {
    background: rgba(10, 10, 10, 0.5);
  }

  .blur-overlay:hover .eye-icon {
    transform: scale(1.1);
    color: #FBBF24;
  }

  .eye-icon {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.1);
    display: flex;
    align-items: center;
    justify-content: center;
    color: rgba(255, 255, 255, 0.8);
    transition: all 0.3s ease;
  }

  .eye-icon svg {
    width: 24px;
    height: 24px;
  }

  .blur-overlay span {
    font-size: 14px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.7);
  }

  .word-chip {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
    transition: all 0.2s ease;
  }

  .mnemonic-container.revealed .word-chip:hover {
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(251, 191, 36, 0.2);
    transform: translateY(-1px);
  }

  .index {
    font-size: 11px;
    font-weight: 700;
    color: #FBBF24;
    background: rgba(251, 191, 36, 0.15);
    padding: 4px 8px;
    border-radius: 6px;
    min-width: 24px;
    text-align: center;
  }

  .word {
    font-family: 'SF Mono', 'Fira Code', monospace;
    font-size: 14px;
    font-weight: 600;
    color: #ffffff;
    letter-spacing: 0.5px;
  }

  /* Copy Button */
  .copy-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 14px 20px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    color: rgba(255, 255, 255, 0.7);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .copy-btn:hover:not(:disabled) {
    background: rgba(251, 191, 36, 0.1);
    border-color: rgba(251, 191, 36, 0.3);
    color: #FBBF24;
  }

  .copy-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* Warning Card */
  .warning-card {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 16px;
    background: rgba(245, 158, 11, 0.08);
    border: 1px solid rgba(245, 158, 11, 0.2);
    border-radius: 14px;
  }

  .warning-icon {
    flex-shrink: 0;
    width: 40px;
    height: 40px;
    background: rgba(245, 158, 11, 0.15);
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #F59E0B;
  }

  .warning-icon svg {
    width: 22px;
    height: 22px;
  }

  .warning-content strong {
    display: block;
    font-size: 14px;
    font-weight: 600;
    color: #ffffff;
    margin-bottom: 4px;
  }

  .warning-content p {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.6);
    line-height: 1.4;
    margin: 0;
  }

  /* Actions */
  .actions {
    display: flex;
    gap: 12px;
    margin-top: auto;
    padding-top: 16px;
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
    position: relative;
    height: 54px;
    background: linear-gradient(135deg, #FBBF24 0%, #F59E0B 100%);
    border: none;
    border-radius: 14px;
    font-size: 15px;
    font-weight: 600;
    color: #000000;
    cursor: pointer;
    overflow: hidden;
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

  .btn-text {
    position: relative;
    z-index: 1;
  }

  .btn-shine {
    position: absolute;
    top: 0;
    left: -100%;
    width: 100%;
    height: 100%;
    background: linear-gradient(
      90deg,
      transparent,
      rgba(255, 255, 255, 0.3),
      transparent
    );
    animation: btnShine 3s ease-in-out infinite;
  }

  @keyframes btnShine {
    0% { left: -100%; }
    50%, 100% { left: 100%; }
  }
</style>
