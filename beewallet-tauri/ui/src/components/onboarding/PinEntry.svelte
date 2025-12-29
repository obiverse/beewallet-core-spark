<script lang="ts">
  import logoSvg from '../../assets/logo.svg';

  interface Props {
    title: string;
    subtitle: string;
    error?: string | null;
    showSuccess?: boolean;
    /** PIN step: 1 = create, 2 = confirm */
    step?: 1 | 2;
    onComplete: (pin: string) => void;
    onBack: () => void;
  }

  let { title, subtitle, error = null, showSuccess = false, step = 1, onComplete, onBack }: Props = $props();

  let pin = $state('');

  function handleDigit(digit: string) {
    if (pin.length < 6) {
      pin += digit;

      if (pin.length === 6) {
        setTimeout(() => onComplete(pin), 150);
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

  $effect(() => {
    if (error) {
      pin = '';
    }
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="pin-entry">
  {#if showSuccess}
    <div class="success-animation">
      <div class="success-container">
        <div class="success-glow"></div>
        <div class="success-ring"></div>
        <div class="success-ring ring-2"></div>
        <div class="success-icon">
          <img src={logoSvg} alt="BeeWallet" class="success-logo" />
        </div>

        <!-- Orbiting sparks -->
        <div class="spark spark-1">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>
          </svg>
        </div>
        <div class="spark spark-2">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>
          </svg>
        </div>
        <div class="spark spark-3">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>
          </svg>
        </div>
      </div>
      <p class="success-text">Perfect!</p>
    </div>
  {:else}
    <!-- Step Indicator -->
    <div class="step-indicator">
      <div class="step-dot" class:active={step >= 1} class:completed={step > 1}>
        {#if step > 1}
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
        {:else}
          1
        {/if}
      </div>
      <div class="step-line" class:active={step >= 2}></div>
      <div class="step-dot" class:active={step >= 2}>
        2
      </div>
    </div>

    <!-- Header -->
    <header>
      <div class="lock-icon">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="3" y="11" width="18" height="11" rx="2"/>
          <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
        </svg>
      </div>
      <h2>{title}</h2>
      <p class="subtitle">{subtitle}</p>
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
        <button class="key" onclick={() => handleDigit(String(digit))}>
          <span class="digit">{digit}</span>
          <div class="key-ripple"></div>
        </button>
      {/each}
      <button class="key empty" disabled aria-hidden="true"></button>
      <button class="key" onclick={() => handleDigit('0')}>
        <span class="digit">0</span>
        <div class="key-ripple"></div>
      </button>
      <button class="key delete" onclick={handleDelete} aria-label="Delete">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 4H8l-7 8 7 8h13a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2z"/>
          <line x1="18" y1="9" x2="12" y2="15"/>
          <line x1="12" y1="9" x2="18" y2="15"/>
        </svg>
      </button>
    </div>

    <!-- Back Button -->
    <button class="back-link" onclick={onBack}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="19" y1="12" x2="5" y2="12"/>
        <polyline points="12 19 5 12 12 5"/>
      </svg>
      Back
    </button>
  {/if}
</div>

<style>
  .pin-entry {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 32px 24px;
    gap: 32px;
    animation: fadeIn 0.3s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* Step Indicator */
  .step-indicator {
    display: flex;
    align-items: center;
    gap: 0;
    margin-bottom: 8px;
  }

  .step-dot {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: 600;
    background: rgba(255, 255, 255, 0.06);
    border: 2px solid rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.3);
    transition: all 0.3s ease;
  }

  .step-dot.active {
    background: rgba(251, 191, 36, 0.15);
    border-color: #FBBF24;
    color: #FBBF24;
  }

  .step-dot.completed {
    background: #10B981;
    border-color: #10B981;
    color: #ffffff;
  }

  .step-line {
    width: 40px;
    height: 2px;
    background: rgba(255, 255, 255, 0.1);
    transition: all 0.3s ease;
  }

  .step-line.active {
    background: #FBBF24;
  }

  /* Header */
  header {
    text-align: center;
  }

  .lock-icon {
    width: 64px;
    height: 64px;
    margin: 0 auto 20px;
    background: linear-gradient(135deg, rgba(251, 191, 36, 0.15) 0%, rgba(251, 191, 36, 0.05) 100%);
    border: 1px solid rgba(251, 191, 36, 0.2);
    border-radius: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #FBBF24;
  }

  .lock-icon svg {
    width: 28px;
    height: 28px;
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
    overflow: hidden;
  }

  .digit {
    font-size: 28px;
    font-weight: 500;
    color: #ffffff;
    position: relative;
    z-index: 1;
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

  .key-ripple {
    position: absolute;
    inset: 0;
    background: radial-gradient(circle at center, rgba(251, 191, 36, 0.3), transparent 70%);
    opacity: 0;
    transition: opacity 0.3s;
  }

  .key:active .key-ripple {
    opacity: 1;
  }

  .key.empty {
    background: transparent;
    border-color: transparent;
    cursor: default;
  }

  .key.delete {
    color: rgba(255, 255, 255, 0.5);
  }

  .key.delete:hover {
    color: #EF4444;
    border-color: rgba(239, 68, 68, 0.3);
    background: rgba(239, 68, 68, 0.1);
  }

  /* Back Link */
  .back-link {
    display: flex;
    align-items: center;
    gap: 8px;
    background: none;
    border: none;
    color: rgba(255, 255, 255, 0.4);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    padding: 12px 16px;
    margin-top: 8px;
    transition: color 0.2s ease;
  }

  .back-link:hover {
    color: rgba(255, 255, 255, 0.8);
  }

  /* Success Animation */
  .success-animation {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 32px;
    animation: fadeIn 0.4s ease-out;
  }

  .success-container {
    position: relative;
    width: 160px;
    height: 160px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .success-glow {
    position: absolute;
    inset: -20px;
    background: radial-gradient(circle, rgba(251, 191, 36, 0.3) 0%, transparent 70%);
    border-radius: 50%;
    animation: glowPulse 2s ease-in-out infinite;
  }

  @keyframes glowPulse {
    0%, 100% { opacity: 0.5; transform: scale(1); }
    50% { opacity: 1; transform: scale(1.1); }
  }

  .success-ring {
    position: absolute;
    width: 130px;
    height: 130px;
    border: 2px dashed rgba(251, 191, 36, 0.3);
    border-radius: 50%;
    animation: spin 8s linear infinite;
  }

  .ring-2 {
    width: 150px;
    height: 150px;
    animation-direction: reverse;
    animation-duration: 12s;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .success-icon {
    position: relative;
    z-index: 1;
  }

  .success-logo {
    width: 80px;
    height: 80px;
    animation: successPulse 1.5s ease-in-out infinite;
    filter: drop-shadow(0 0 24px rgba(251, 191, 36, 0.5));
  }

  @keyframes successPulse {
    0%, 100% { transform: scale(1); }
    50% { transform: scale(1.08); }
  }

  .spark {
    position: absolute;
    color: #FBBF24;
    animation: orbit 3s linear infinite;
    filter: drop-shadow(0 0 6px rgba(251, 191, 36, 0.6));
  }

  .spark-1 { animation-delay: 0s; }
  .spark-2 { animation-delay: 1s; }
  .spark-3 { animation-delay: 2s; }

  @keyframes orbit {
    0% { transform: rotate(0deg) translateX(70px) rotate(0deg); }
    100% { transform: rotate(360deg) translateX(70px) rotate(-360deg); }
  }

  .success-text {
    font-size: 24px;
    font-weight: 600;
    color: #10B981;
  }
</style>
