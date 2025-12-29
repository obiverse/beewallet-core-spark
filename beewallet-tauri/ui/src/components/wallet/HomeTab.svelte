<script lang="ts">
  import { Card } from '../ui';
</script>

<div class="home-tab">
  <!-- Network Info Card -->
  <Card>
    <div class="card-header">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/>
        <line x1="2" y1="12" x2="22" y2="12"/>
        <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
      </svg>
      <h3>Network Info</h3>
    </div>
    <div class="info-rows">
      <div class="info-row">
        <span class="info-label">Network</span>
        <span class="info-value">Spark Regtest</span>
      </div>
      <div class="info-row">
        <span class="info-label">Backend</span>
        <span class="info-value">beewallet-core-spark</span>
      </div>
      <div class="info-row">
        <span class="info-label">Faucet</span>
        <a
          href="https://app.lightspark.com/regtest-faucet"
          target="_blank"
          rel="noopener"
          class="info-link"
        >
          Get Test Coins
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
            <polyline points="15 3 21 3 21 9"/>
            <line x1="10" y1="14" x2="21" y2="3"/>
          </svg>
        </a>
      </div>
    </div>
  </Card>

  <!-- Recent Activity -->
  <div class="section">
    <div class="section-header">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M22 12h-4l-3 9L9 3l-3 9H2"/>
      </svg>
      <h3>Recent Activity</h3>
    </div>
    <div class="empty-state">
      <div class="empty-icon">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <circle cx="12" cy="12" r="10"/>
          <path d="M8 12h8"/>
        </svg>
      </div>
      <p class="empty-title">No transactions yet</p>
      <p class="empty-hint">Send or receive some sats to get started</p>
    </div>
  </div>
</div>

<style>
  .home-tab {
    animation: fadeIn 0.3s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: var(--space-4, 16px);
    color: var(--color-text-tertiary, rgba(255, 255, 255, 0.5));
  }

  .card-header h3 {
    font-size: var(--text-base, 14px);
    font-weight: var(--font-semibold, 600);
    color: var(--color-text, #ffffff);
  }

  .info-rows {
    display: flex;
    flex-direction: column;
  }

  .info-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-3, 12px) 0;
    border-bottom: 1px solid var(--color-border, rgba(255, 255, 255, 0.04));
  }

  .info-row:last-child {
    border-bottom: none;
    padding-bottom: 0;
  }

  .info-row:first-child {
    padding-top: 0;
  }

  .info-label {
    font-size: var(--text-base, 14px);
    color: var(--color-text-tertiary, rgba(255, 255, 255, 0.5));
  }

  .info-value {
    font-size: var(--text-base, 14px);
    font-weight: var(--font-medium, 500);
    color: var(--color-text, #ffffff);
  }

  .info-link {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-base, 14px);
    font-weight: var(--font-medium, 500);
    color: var(--color-primary, #FBBF24);
    text-decoration: none;
    transition: color var(--transition-base, 0.2s ease);
  }

  .info-link:hover {
    color: var(--color-primary-light, #FDE68A);
  }

  /* Section */
  .section {
    margin-top: var(--space-5, 20px);
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: var(--space-4, 16px);
    color: var(--color-text-tertiary, rgba(255, 255, 255, 0.5));
  }

  .section-header h3 {
    font-size: var(--text-base, 14px);
    font-weight: var(--font-semibold, 600);
    color: var(--color-text, #ffffff);
  }

  /* Empty State */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: var(--space-10, 40px) var(--space-5, 20px);
    text-align: center;
  }

  .empty-icon {
    color: rgba(255, 255, 255, 0.15);
    margin-bottom: var(--space-4, 16px);
  }

  .empty-title {
    font-size: var(--text-lg, 15px);
    font-weight: var(--font-medium, 500);
    color: var(--color-text-tertiary, rgba(255, 255, 255, 0.5));
    margin: 0;
  }

  .empty-hint {
    font-size: var(--text-md, 13px);
    color: var(--color-text-muted, rgba(255, 255, 255, 0.3));
    margin: 6px 0 0;
  }
</style>
