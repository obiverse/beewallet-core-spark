<script lang="ts">
  import logoSvg from '../../assets/logo.svg';

  interface Props {
    error?: string | null;
    onRetry: () => void;
    onBack: () => void;
    onStartOver: () => void;
  }

  let { error = null, onRetry, onBack, onStartOver }: Props = $props();
</script>

<div class="sealing">
  {#if error}
    <div class="error-state">
      <div class="error-icon-container">
        <div class="error-icon">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="8" x2="12" y2="12"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
        </div>
      </div>

      <h2>Something went wrong</h2>

      <div class="error-details">
        <p>{error}</p>
      </div>

      <div class="actions">
        <button class="btn-secondary" onclick={onBack}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="19" y1="12" x2="5" y2="12"/>
            <polyline points="12 19 5 12 12 5"/>
          </svg>
          Back to PIN
        </button>
        <button class="btn-primary" onclick={onRetry}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="23 4 23 10 17 10"/>
            <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
          </svg>
          Try Again
        </button>
      </div>

      <button class="start-over" onclick={onStartOver}>
        Start Over
      </button>
    </div>
  {:else}
    <div class="loading-state">
      <!-- Vault Animation -->
      <div class="vault-container">
        <div class="vault-glow"></div>
        <div class="vault-ring"></div>
        <div class="vault-ring ring-2"></div>

        <div class="vault-door">
          <div class="vault-lock">
            <img src={logoSvg} alt="BeeWallet" class="vault-logo" />
          </div>
        </div>

        <!-- Encryption particles -->
        <div class="particle p1"></div>
        <div class="particle p2"></div>
        <div class="particle p3"></div>
        <div class="particle p4"></div>
        <div class="particle p5"></div>
        <div class="particle p6"></div>
      </div>

      <div class="loading-text">
        <h2>Sealing your vault...</h2>
        <p>Military-grade encryption</p>
      </div>

      <div class="progress-dots">
        <span class="dot"></span>
        <span class="dot"></span>
        <span class="dot"></span>
      </div>
    </div>
  {/if}
</div>

<style>
  .sealing {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 32px;
  }

  /* Loading State */
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 32px;
    animation: fadeIn 0.4s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: scale(0.95); }
    to { opacity: 1; transform: scale(1); }
  }

  .vault-container {
    position: relative;
    width: 180px;
    height: 180px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .vault-glow {
    position: absolute;
    inset: -30px;
    background: radial-gradient(circle, rgba(251, 191, 36, 0.25) 0%, transparent 70%);
    border-radius: 50%;
    animation: glowPulse 2.5s ease-in-out infinite;
  }

  @keyframes glowPulse {
    0%, 100% { opacity: 0.5; transform: scale(1); }
    50% { opacity: 1; transform: scale(1.15); }
  }

  .vault-ring {
    position: absolute;
    width: 150px;
    height: 150px;
    border: 2px solid rgba(251, 191, 36, 0.2);
    border-radius: 50%;
    animation: ringRotate 4s linear infinite;
  }

  .ring-2 {
    width: 170px;
    height: 170px;
    border-style: dashed;
    animation-direction: reverse;
    animation-duration: 6s;
  }

  @keyframes ringRotate {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .vault-door {
    position: relative;
    width: 100px;
    height: 100px;
    background: linear-gradient(135deg, rgba(255, 255, 255, 0.08) 0%, rgba(255, 255, 255, 0.02) 100%);
    border: 2px solid rgba(251, 191, 36, 0.4);
    border-radius: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: vaultPulse 2s ease-in-out infinite;
    box-shadow:
      0 0 40px rgba(251, 191, 36, 0.2),
      inset 0 0 20px rgba(251, 191, 36, 0.05);
  }

  @keyframes vaultPulse {
    0%, 100% { transform: scale(1); }
    50% { transform: scale(1.05); }
  }

  .vault-logo {
    width: 50px;
    height: 50px;
    filter: drop-shadow(0 0 16px rgba(251, 191, 36, 0.5));
    animation: logoRotate 3s ease-in-out infinite;
  }

  @keyframes logoRotate {
    0%, 100% { transform: rotate(0deg); }
    25% { transform: rotate(-5deg); }
    75% { transform: rotate(5deg); }
  }

  /* Particles */
  .particle {
    position: absolute;
    width: 4px;
    height: 4px;
    background: #FBBF24;
    border-radius: 50%;
    animation: particleFloat 3s ease-in-out infinite;
  }

  .p1 { top: 20%; left: 10%; animation-delay: 0s; }
  .p2 { top: 80%; left: 15%; animation-delay: 0.5s; }
  .p3 { top: 30%; right: 10%; animation-delay: 1s; }
  .p4 { top: 70%; right: 15%; animation-delay: 1.5s; }
  .p5 { top: 50%; left: 5%; animation-delay: 2s; }
  .p6 { top: 50%; right: 5%; animation-delay: 2.5s; }

  @keyframes particleFloat {
    0%, 100% {
      opacity: 0;
      transform: translate(0, 0) scale(0);
    }
    50% {
      opacity: 1;
      transform: translate(10px, -20px) scale(1);
    }
  }

  .loading-text h2 {
    font-size: 24px;
    font-weight: 700;
    color: #ffffff;
    margin-bottom: 8px;
  }

  .loading-text p {
    font-size: 15px;
    color: rgba(255, 255, 255, 0.5);
  }

  .progress-dots {
    display: flex;
    gap: 8px;
  }

  .progress-dots .dot {
    width: 8px;
    height: 8px;
    background: rgba(251, 191, 36, 0.3);
    border-radius: 50%;
    animation: dotBounce 1.4s ease-in-out infinite;
  }

  .progress-dots .dot:nth-child(1) { animation-delay: 0s; }
  .progress-dots .dot:nth-child(2) { animation-delay: 0.2s; }
  .progress-dots .dot:nth-child(3) { animation-delay: 0.4s; }

  @keyframes dotBounce {
    0%, 80%, 100% {
      transform: scale(1);
      background: rgba(251, 191, 36, 0.3);
    }
    40% {
      transform: scale(1.3);
      background: #FBBF24;
    }
  }

  /* Error State */
  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 24px;
    max-width: 340px;
    animation: fadeIn 0.4s ease-out;
  }

  .error-icon-container {
    width: 80px;
    height: 80px;
  }

  .error-icon {
    width: 100%;
    height: 100%;
    background: linear-gradient(135deg, rgba(239, 68, 68, 0.15) 0%, rgba(239, 68, 68, 0.05) 100%);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #EF4444;
  }

  .error-icon svg {
    width: 36px;
    height: 36px;
  }

  .error-state h2 {
    font-size: 24px;
    font-weight: 700;
    color: #ffffff;
  }

  .error-details {
    width: 100%;
    background: rgba(239, 68, 68, 0.08);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 14px;
    padding: 16px;
  }

  .error-details p {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.7);
    font-family: 'SF Mono', 'Fira Code', monospace;
    word-break: break-all;
    margin: 0;
    line-height: 1.5;
  }

  .actions {
    display: flex;
    gap: 12px;
    width: 100%;
  }

  .btn-secondary {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 16px;
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 14px;
    font-size: 14px;
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
    gap: 8px;
    padding: 16px;
    background: linear-gradient(135deg, #FBBF24 0%, #F59E0B 100%);
    border: none;
    border-radius: 14px;
    font-size: 14px;
    font-weight: 600;
    color: #000000;
    cursor: pointer;
    transition: all 0.3s ease;
    box-shadow: 0 4px 16px rgba(251, 191, 36, 0.25);
  }

  .btn-primary:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 24px rgba(251, 191, 36, 0.35);
  }

  .start-over {
    background: none;
    border: none;
    color: rgba(255, 255, 255, 0.4);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    padding: 12px;
    transition: color 0.2s ease;
  }

  .start-over:hover {
    color: rgba(255, 255, 255, 0.7);
    text-decoration: underline;
  }
</style>
