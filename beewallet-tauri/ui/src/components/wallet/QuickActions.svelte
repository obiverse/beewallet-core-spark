<script lang="ts">
  interface Props {
    activeTab: 'home' | 'send' | 'receive';
    syncing?: boolean;
    onSend: () => void;
    onReceive: () => void;
    onSync: () => void;
  }

  let { activeTab, syncing = false, onSend, onReceive, onSync }: Props = $props();
</script>

<div class="quick-actions">
  <button class="action-btn" class:active={activeTab === 'send'} onclick={onSend}>
    <div class="action-icon send">
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
        <line x1="12" y1="19" x2="12" y2="5"/>
        <polyline points="5 12 12 5 19 12"/>
      </svg>
    </div>
    <span>Send</span>
  </button>

  <button class="action-btn" class:active={activeTab === 'receive'} onclick={onReceive}>
    <div class="action-icon receive">
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
        <line x1="12" y1="5" x2="12" y2="19"/>
        <polyline points="19 12 12 19 5 12"/>
      </svg>
    </div>
    <span>Receive</span>
  </button>

  <button class="action-btn" onclick={onSync} disabled={syncing}>
    <div class="action-icon sync" class:spinning={syncing}>
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="23 4 23 10 17 10"/>
        <polyline points="1 20 1 14 7 14"/>
        <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
      </svg>
    </div>
    <span>Sync</span>
  </button>
</div>

<style>
  .quick-actions {
    display: flex;
    justify-content: center;
    gap: 32px;
    padding: 28px var(--space-5, 20px);
  }

  .action-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    background: none;
    border: none;
    cursor: pointer;
    padding: 8px;
    transition: all var(--transition-base, 0.2s ease);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .action-icon {
    width: 56px;
    height: 56px;
    border-radius: var(--radius-xl, 16px);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--transition-base, 0.2s ease);
  }

  .action-icon.send {
    background: rgba(251, 191, 36, 0.12);
    color: var(--color-primary, #FBBF24);
    border: 1px solid rgba(251, 191, 36, 0.2);
  }

  .action-icon.receive {
    background: rgba(16, 185, 129, 0.12);
    color: var(--color-success, #10B981);
    border: 1px solid rgba(16, 185, 129, 0.2);
  }

  .action-icon.sync {
    background: rgba(59, 130, 246, 0.12);
    color: var(--color-info, #3B82F6);
    border: 1px solid rgba(59, 130, 246, 0.2);
  }

  .action-btn:hover:not(:disabled) .action-icon {
    transform: scale(1.08);
  }

  .action-btn.active .action-icon {
    transform: scale(1.08);
  }

  .action-btn span {
    font-size: var(--text-md, 13px);
    font-weight: var(--font-medium, 500);
    color: var(--color-text-tertiary, rgba(255, 255, 255, 0.6));
  }

  .spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
