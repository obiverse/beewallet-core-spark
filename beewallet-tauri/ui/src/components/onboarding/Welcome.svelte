<script lang="ts">
  import logoSvg from '../../assets/logo.svg';

  interface Props {
    onCreate: () => void;
    onRestore: () => void;
  }

  let { onCreate, onRestore }: Props = $props();
</script>

<div class="welcome">
  <!-- Ambient Background -->
  <div class="ambient-glow"></div>

  <!-- Hero Section -->
  <div class="hero">
    <div class="logo-container">
      <div class="logo-ring"></div>
      <div class="logo-ring ring-2"></div>
      <img src={logoSvg} alt="BeeWallet" class="logo" />
    </div>

    <div class="branding">
      <h1>BeeWallet</h1>
      <p class="tagline">Your Sovereign Bitcoin Hive</p>
    </div>
  </div>

  <!-- Features Section -->
  <div class="features">
    <div class="feature-card">
      <div class="feature-icon lightning">
        <svg viewBox="0 0 24 24" fill="currentColor">
          <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>
        </svg>
      </div>
      <div class="feature-content">
        <h3>Instant Payments</h3>
        <p>Lightning-fast via Spark L2</p>
      </div>
    </div>

    <div class="feature-card">
      <div class="feature-icon security">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="3" y="11" width="18" height="11" rx="2"/>
          <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
        </svg>
      </div>
      <div class="feature-content">
        <h3>Self-Custody</h3>
        <p>Your keys, your coins</p>
      </div>
    </div>

    <div class="feature-card">
      <div class="feature-icon shield">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
          <path d="M9 12l2 2 4-4"/>
        </svg>
      </div>
      <div class="feature-content">
        <h3>Military-Grade</h3>
        <p>AES-256 + Argon2id encryption</p>
      </div>
    </div>
  </div>

  <!-- Action Buttons -->
  <div class="actions">
    <button class="btn-primary" onclick={onCreate}>
      <span class="btn-content">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"/>
          <line x1="12" y1="8" x2="12" y2="16"/>
          <line x1="8" y1="12" x2="16" y2="12"/>
        </svg>
        Create New Wallet
      </span>
      <div class="btn-shine"></div>
    </button>

    <button class="btn-secondary" onclick={onRestore}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="1 4 1 10 7 10"/>
        <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"/>
      </svg>
      Restore Existing Wallet
    </button>
  </div>

  <!-- Network Badge -->
  <div class="network-indicator">
    <span class="pulse"></span>
    <span class="network-text">Spark Regtest</span>
  </div>
</div>

<style>
  .welcome {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    padding: 48px 24px 32px;
    position: relative;
    overflow: hidden;
  }

  /* Ambient Background Glow */
  .ambient-glow {
    position: absolute;
    top: -200px;
    left: 50%;
    transform: translateX(-50%);
    width: 600px;
    height: 600px;
    background: radial-gradient(
      circle,
      rgba(251, 191, 36, 0.15) 0%,
      rgba(251, 191, 36, 0.05) 40%,
      transparent 70%
    );
    pointer-events: none;
    animation: ambientPulse 4s ease-in-out infinite;
  }

  @keyframes ambientPulse {
    0%, 100% { opacity: 0.6; transform: translateX(-50%) scale(1); }
    50% { opacity: 1; transform: translateX(-50%) scale(1.1); }
  }

  /* Hero Section */
  .hero {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 32px;
    padding: 40px 0;
    position: relative;
    z-index: 1;
  }

  .logo-container {
    position: relative;
    width: 160px;
    height: 160px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .logo-ring {
    position: absolute;
    width: 140px;
    height: 140px;
    border: 1px solid rgba(251, 191, 36, 0.2);
    border-radius: 50%;
    animation: ringPulse 3s ease-in-out infinite;
  }

  .ring-2 {
    width: 170px;
    height: 170px;
    animation-delay: 0.5s;
    border-color: rgba(251, 191, 36, 0.1);
  }

  @keyframes ringPulse {
    0%, 100% { transform: scale(1); opacity: 1; }
    50% { transform: scale(1.05); opacity: 0.5; }
  }

  .logo {
    width: 100px;
    height: 100px;
    filter: drop-shadow(0 0 30px rgba(251, 191, 36, 0.4));
    animation: logoFloat 4s ease-in-out infinite;
    position: relative;
    z-index: 2;
  }

  @keyframes logoFloat {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-8px); }
  }

  .branding {
    text-align: center;
  }

  .branding h1 {
    font-size: 40px;
    font-weight: 700;
    letter-spacing: -1px;
    background: linear-gradient(135deg, #FBBF24 0%, #FDE68A 50%, #FBBF24 100%);
    background-size: 200% auto;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    animation: shimmer 3s linear infinite;
    margin-bottom: 8px;
  }

  @keyframes shimmer {
    0% { background-position: 0% center; }
    100% { background-position: 200% center; }
  }

  .tagline {
    font-size: 16px;
    color: rgba(255, 255, 255, 0.6);
    font-weight: 400;
    letter-spacing: 0.5px;
  }

  /* Features Section */
  .features {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 32px;
    position: relative;
    z-index: 1;
  }

  .feature-card {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px 20px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 16px;
    backdrop-filter: blur(10px);
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .feature-card:hover {
    background: rgba(255, 255, 255, 0.05);
    border-color: rgba(251, 191, 36, 0.2);
    transform: translateX(4px);
  }

  .feature-icon {
    width: 48px;
    height: 48px;
    border-radius: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .feature-icon svg {
    width: 24px;
    height: 24px;
  }

  .feature-icon.lightning {
    background: linear-gradient(135deg, rgba(251, 191, 36, 0.2) 0%, rgba(251, 191, 36, 0.1) 100%);
    color: #FBBF24;
  }

  .feature-icon.security {
    background: linear-gradient(135deg, rgba(16, 185, 129, 0.2) 0%, rgba(16, 185, 129, 0.1) 100%);
    color: #10B981;
  }

  .feature-icon.shield {
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.2) 0%, rgba(59, 130, 246, 0.1) 100%);
    color: #3B82F6;
  }

  .feature-content h3 {
    font-size: 15px;
    font-weight: 600;
    color: #ffffff;
    margin-bottom: 2px;
  }

  .feature-content p {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.5);
    font-weight: 400;
  }

  /* Action Buttons */
  .actions {
    display: flex;
    flex-direction: column;
    gap: 12px;
    position: relative;
    z-index: 1;
  }

  .btn-primary {
    position: relative;
    width: 100%;
    height: 56px;
    background: linear-gradient(135deg, #FBBF24 0%, #F59E0B 100%);
    border: none;
    border-radius: 16px;
    font-size: 16px;
    font-weight: 600;
    color: #000000;
    cursor: pointer;
    overflow: hidden;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow:
      0 4px 16px rgba(251, 191, 36, 0.3),
      0 0 0 1px rgba(251, 191, 36, 0.1) inset;
  }

  .btn-primary:hover {
    transform: translateY(-2px);
    box-shadow:
      0 8px 32px rgba(251, 191, 36, 0.4),
      0 0 0 1px rgba(255, 255, 255, 0.2) inset;
  }

  .btn-primary:active {
    transform: translateY(0);
  }

  .btn-content {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    position: relative;
    z-index: 1;
  }

  .btn-content svg {
    width: 20px;
    height: 20px;
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

  .btn-secondary {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    width: 100%;
    height: 56px;
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 16px;
    font-size: 16px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.8);
    cursor: pointer;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .btn-secondary svg {
    width: 20px;
    height: 20px;
  }

  .btn-secondary:hover {
    background: rgba(255, 255, 255, 0.05);
    border-color: rgba(251, 191, 36, 0.3);
    color: #ffffff;
  }

  /* Network Indicator */
  .network-indicator {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    margin-top: 24px;
    position: relative;
    z-index: 1;
  }

  .pulse {
    width: 8px;
    height: 8px;
    background: #10B981;
    border-radius: 50%;
    animation: pulse 2s ease-in-out infinite;
    box-shadow: 0 0 8px rgba(16, 185, 129, 0.5);
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.5; transform: scale(0.9); }
  }

  .network-text {
    font-size: 12px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.4);
    text-transform: uppercase;
    letter-spacing: 1px;
  }
</style>
