<script lang="ts">
  import logoSvg from '../../assets/logo.svg';

  interface Props {
    mobinumber: string | null;
    onEnter: () => void;
  }

  let { mobinumber, onEnter }: Props = $props();

  let copied = $state(false);

  // Format mobinumber for display (XXX-XXX-XXX-XXX)
  function formatMobinumber(mobi: string): string {
    if (mobi.includes('-')) return mobi;
    const clean = mobi.replace(/\D/g, '');
    return clean.replace(/(\d{3})(\d{3})(\d{3})(\d{3})/, '$1-$2-$3-$4');
  }

  function copyToClipboard() {
    if (mobinumber) {
      navigator.clipboard.writeText(mobinumber);
      copied = true;
      setTimeout(() => copied = false, 2000);
    }
  }
</script>

<div class="identity">
  <!-- Success Animation -->
  <div class="success-hero">
    <div class="hero-glow"></div>
    <div class="hero-ring"></div>
    <div class="hero-ring ring-2"></div>

    <div class="hexagon-container">
      <div class="hexagon">
        <img src={logoSvg} alt="BeeWallet" class="hero-logo" />
      </div>
    </div>

    <!-- Celebration particles -->
    <div class="confetti c1"></div>
    <div class="confetti c2"></div>
    <div class="confetti c3"></div>
    <div class="confetti c4"></div>
    <div class="confetti c5"></div>
    <div class="confetti c6"></div>
  </div>

  <!-- Title -->
  <div class="title-section">
    <h1>Your Hive is Ready</h1>
    <p class="subtitle">Welcome to your sovereign Bitcoin wallet</p>
  </div>

  <!-- Mobinumber Card -->
  <div class="mobinumber-card">
    <div class="card-header">
      <span class="label">Your Mobinumber</span>
      <span class="badge">Unique ID</span>
    </div>

    <button class="mobinumber-display" onclick={copyToClipboard} title="Click to copy">
      <span class="mobinumber-text">
        {mobinumber ? formatMobinumber(mobinumber) : '---'}
      </span>
      <span class="copy-indicator" class:copied>
        {#if copied}
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
        {:else}
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
          </svg>
        {/if}
      </span>
    </button>

    <p class="hint">Your unique identity derived from your seed</p>
  </div>

  <!-- Features -->
  <div class="features">
    <div class="feature">
      <div class="feature-icon lightning">
        <svg viewBox="0 0 24 24" fill="currentColor">
          <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>
        </svg>
      </div>
      <div class="feature-text">
        <strong>Instant Payments</strong>
        <span>Lightning-fast via Spark L2</span>
      </div>
    </div>

    <div class="feature">
      <div class="feature-icon security">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="3" y="11" width="18" height="11" rx="2"/>
          <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
        </svg>
      </div>
      <div class="feature-text">
        <strong>Military-Grade</strong>
        <span>AES-256 + Argon2id encryption</span>
      </div>
    </div>

    <div class="feature">
      <div class="feature-icon sovereignty">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"/>
          <line x1="2" y1="12" x2="22" y2="12"/>
          <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
        </svg>
      </div>
      <div class="feature-text">
        <strong>Self-Sovereign</strong>
        <span>Your keys, your coins</span>
      </div>
    </div>
  </div>

  <!-- Enter Button -->
  <button class="enter-button" onclick={onEnter}>
    <span class="btn-content">
      Enter Your Hive
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="5" y1="12" x2="19" y2="12"/>
        <polyline points="12 5 19 12 12 19"/>
      </svg>
    </span>
    <div class="btn-shine"></div>
  </button>
</div>

<style>
  .identity {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 48px 24px 32px;
    gap: 28px;
    animation: fadeIn 0.5s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(20px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* Success Hero */
  .success-hero {
    position: relative;
    width: 200px;
    height: 200px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .hero-glow {
    position: absolute;
    inset: -40px;
    background: radial-gradient(circle, rgba(251, 191, 36, 0.3) 0%, transparent 70%);
    border-radius: 50%;
    animation: glowPulse 3s ease-in-out infinite;
  }

  @keyframes glowPulse {
    0%, 100% { opacity: 0.5; transform: scale(1); }
    50% { opacity: 1; transform: scale(1.1); }
  }

  .hero-ring {
    position: absolute;
    width: 160px;
    height: 160px;
    border: 2px solid rgba(251, 191, 36, 0.2);
    border-radius: 50%;
    animation: ringRotate 8s linear infinite;
  }

  .ring-2 {
    width: 180px;
    height: 180px;
    border-style: dashed;
    animation-direction: reverse;
    animation-duration: 12s;
  }

  @keyframes ringRotate {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .hexagon-container {
    position: relative;
    z-index: 1;
  }

  .hexagon {
    width: 100px;
    height: 110px;
    background: linear-gradient(135deg, #FBBF24 0%, #F59E0B 100%);
    clip-path: polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%);
    display: flex;
    align-items: center;
    justify-content: center;
    animation: hexFloat 3s ease-in-out infinite;
    box-shadow: 0 0 40px rgba(251, 191, 36, 0.4);
  }

  @keyframes hexFloat {
    0%, 100% { transform: translateY(0) rotate(0deg); }
    50% { transform: translateY(-8px) rotate(2deg); }
  }

  .hero-logo {
    width: 50px;
    height: 50px;
    filter: brightness(0);
  }

  /* Confetti particles */
  .confetti {
    position: absolute;
    width: 6px;
    height: 6px;
    border-radius: 2px;
    animation: confettiFall 4s ease-in-out infinite;
  }

  .c1 { background: #FBBF24; top: 10%; left: 20%; animation-delay: 0s; }
  .c2 { background: #10B981; top: 20%; right: 15%; animation-delay: 0.5s; }
  .c3 { background: #3B82F6; top: 40%; left: 10%; animation-delay: 1s; }
  .c4 { background: #FBBF24; top: 60%; right: 20%; animation-delay: 1.5s; }
  .c5 { background: #10B981; top: 80%; left: 25%; animation-delay: 2s; }
  .c6 { background: #3B82F6; top: 70%; right: 10%; animation-delay: 2.5s; }

  @keyframes confettiFall {
    0%, 100% {
      opacity: 0;
      transform: translate(0, 0) rotate(0deg) scale(0);
    }
    50% {
      opacity: 1;
      transform: translate(10px, 20px) rotate(180deg) scale(1);
    }
  }

  /* Title Section */
  .title-section {
    text-align: center;
  }

  h1 {
    font-size: 32px;
    font-weight: 700;
    letter-spacing: -0.5px;
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

  .subtitle {
    font-size: 16px;
    color: rgba(255, 255, 255, 0.5);
  }

  /* Mobinumber Card */
  .mobinumber-card {
    width: 100%;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 20px;
    padding: 20px;
  }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  .label {
    font-size: 12px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.5);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .badge {
    font-size: 10px;
    font-weight: 600;
    color: #FBBF24;
    background: rgba(251, 191, 36, 0.15);
    padding: 4px 8px;
    border-radius: 6px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .mobinumber-display {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 16px;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .mobinumber-display:hover {
    background: rgba(0, 0, 0, 0.4);
    border-color: rgba(251, 191, 36, 0.3);
  }

  .mobinumber-text {
    font-size: 20px;
    font-weight: 600;
    font-family: 'SF Mono', 'Fira Code', monospace;
    color: #FBBF24;
    letter-spacing: 1px;
  }

  .copy-indicator {
    color: rgba(255, 255, 255, 0.4);
    transition: all 0.2s ease;
  }

  .copy-indicator.copied {
    color: #10B981;
  }

  .hint {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.4);
    text-align: center;
    margin-top: 12px;
  }

  /* Features */
  .features {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .feature {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 16px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 14px;
    transition: all 0.2s ease;
  }

  .feature:hover {
    background: rgba(255, 255, 255, 0.05);
    border-color: rgba(251, 191, 36, 0.1);
    transform: translateX(4px);
  }

  .feature-icon {
    width: 44px;
    height: 44px;
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .feature-icon svg {
    width: 22px;
    height: 22px;
  }

  .feature-icon.lightning {
    background: linear-gradient(135deg, rgba(251, 191, 36, 0.2) 0%, rgba(251, 191, 36, 0.1) 100%);
    color: #FBBF24;
  }

  .feature-icon.security {
    background: linear-gradient(135deg, rgba(16, 185, 129, 0.2) 0%, rgba(16, 185, 129, 0.1) 100%);
    color: #10B981;
  }

  .feature-icon.sovereignty {
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.2) 0%, rgba(59, 130, 246, 0.1) 100%);
    color: #3B82F6;
  }

  .feature-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .feature-text strong {
    font-size: 14px;
    font-weight: 600;
    color: #ffffff;
  }

  .feature-text span {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.5);
  }

  /* Enter Button */
  .enter-button {
    position: relative;
    width: 100%;
    height: 58px;
    background: linear-gradient(135deg, #FBBF24 0%, #F59E0B 100%);
    border: none;
    border-radius: 16px;
    font-size: 17px;
    font-weight: 600;
    color: #000000;
    cursor: pointer;
    overflow: hidden;
    margin-top: auto;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow:
      0 4px 20px rgba(251, 191, 36, 0.35),
      0 0 0 1px rgba(251, 191, 36, 0.1) inset;
  }

  .enter-button:hover {
    transform: translateY(-3px);
    box-shadow:
      0 8px 32px rgba(251, 191, 36, 0.45),
      0 0 0 1px rgba(255, 255, 255, 0.2) inset;
  }

  .enter-button:active {
    transform: translateY(-1px);
  }

  .btn-content {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
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
