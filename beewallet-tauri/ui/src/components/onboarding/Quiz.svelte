<script lang="ts">
  interface Props {
    mnemonic: string[];
    indices: number[];
    inputs: string[];
    onConfirm: () => void;
    onBack: () => void;
    isValid: boolean;
  }

  let { mnemonic, indices, inputs = $bindable(), onConfirm, onBack, isValid }: Props = $props();

  function isWordCorrect(index: number): boolean {
    const quizIndex = indices.indexOf(index);
    if (quizIndex === -1) return false;
    return inputs[quizIndex].toLowerCase().trim() === mnemonic[index].toLowerCase();
  }

  function getInputState(i: number): 'empty' | 'correct' | 'incorrect' {
    if (!inputs[i].trim()) return 'empty';
    const mnemonicIndex = indices[i];
    return inputs[i].toLowerCase().trim() === mnemonic[mnemonicIndex].toLowerCase()
      ? 'correct'
      : 'incorrect';
  }
</script>

<div class="quiz">
  <!-- Header -->
  <header>
    <div class="icon-container">
      <div class="icon-bg">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
        </svg>
      </div>
    </div>
    <h2>Verify Your Backup</h2>
    <p class="subtitle">Enter the words at these positions to confirm you've saved your phrase.</p>
  </header>

  <!-- Input Fields -->
  <div class="inputs">
    {#each indices as wordIndex, i}
      {@const state = getInputState(i)}
      <div class="input-group" class:correct={state === 'correct'} class:incorrect={state === 'incorrect'}>
        <label for="word-{i}">
          Word #{wordIndex + 1}
          {#if state === 'correct'}
            <span class="status-badge correct">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
            </span>
          {:else if state === 'incorrect'}
            <span class="status-badge incorrect">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                <line x1="18" y1="6" x2="6" y2="18"/>
                <line x1="6" y1="6" x2="18" y2="18"/>
              </svg>
            </span>
          {/if}
        </label>
        <div class="input-wrapper">
          <input
            id="word-{i}"
            type="text"
            bind:value={inputs[i]}
            placeholder="Enter word"
            autocomplete="off"
            autocapitalize="off"
            spellcheck="false"
          />
        </div>
      </div>
    {/each}
  </div>

  <!-- Help Section -->
  <div class="help-card">
    <details>
      <summary>
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"/>
          <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
          <line x1="12" y1="17" x2="12.01" y2="17"/>
        </svg>
        How to store your seed phrase safely
      </summary>
      <div class="help-content">
        <div class="do">
          <div class="do-header">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            Do
          </div>
          <ul>
            <li>Write it on paper</li>
            <li>Store in a safe place</li>
            <li>Consider metal backup</li>
          </ul>
        </div>
        <div class="dont">
          <div class="dont-header">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
            Don't
          </div>
          <ul>
            <li>Screenshot or photograph</li>
            <li>Store digitally (cloud, notes)</li>
            <li>Share with anyone</li>
          </ul>
        </div>
      </div>
    </details>
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
    <button class="btn-primary" onclick={onConfirm} disabled={!isValid}>
      <span class="btn-text">Continue</span>
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="5" y1="12" x2="19" y2="12"/>
        <polyline points="12 5 19 12 12 19"/>
      </svg>
    </button>
  </div>
</div>

<style>
  .quiz {
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
    width: 72px;
    height: 72px;
    margin: 0 auto 20px;
  }

  .icon-bg {
    width: 100%;
    height: 100%;
    background: linear-gradient(135deg, rgba(16, 185, 129, 0.15) 0%, rgba(16, 185, 129, 0.05) 100%);
    border: 1px solid rgba(16, 185, 129, 0.2);
    border-radius: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #10B981;
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
    line-height: 1.5;
  }

  /* Inputs */
  .inputs {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.6);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .status-badge {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
  }

  .status-badge.correct {
    background: rgba(16, 185, 129, 0.2);
    color: #10B981;
  }

  .status-badge.incorrect {
    background: rgba(239, 68, 68, 0.2);
    color: #EF4444;
  }

  .input-wrapper {
    position: relative;
  }

  input {
    width: 100%;
    padding: 16px;
    background: rgba(255, 255, 255, 0.04);
    border: 2px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    font-size: 16px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    color: #ffffff;
    transition: all 0.2s ease;
  }

  input::placeholder {
    color: rgba(255, 255, 255, 0.3);
  }

  input:focus {
    outline: none;
    border-color: rgba(251, 191, 36, 0.5);
    background: rgba(255, 255, 255, 0.06);
  }

  .input-group.correct input {
    border-color: rgba(16, 185, 129, 0.5);
    background: rgba(16, 185, 129, 0.08);
  }

  .input-group.incorrect input {
    border-color: rgba(239, 68, 68, 0.5);
    background: rgba(239, 68, 68, 0.08);
  }

  /* Help Card */
  .help-card {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 16px;
    overflow: hidden;
  }

  details summary {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 16px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.6);
    transition: all 0.2s ease;
    list-style: none;
  }

  details summary::-webkit-details-marker {
    display: none;
  }

  details summary:hover {
    background: rgba(255, 255, 255, 0.03);
    color: rgba(255, 255, 255, 0.8);
  }

  details[open] summary {
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .help-content {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    padding: 16px;
  }

  .do, .dont {
    font-size: 13px;
  }

  .do-header, .dont-header {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
    margin-bottom: 10px;
  }

  .do-header {
    color: #10B981;
  }

  .dont-header {
    color: #EF4444;
  }

  ul {
    margin: 0;
    padding-left: 20px;
  }

  li {
    margin-bottom: 6px;
    color: rgba(255, 255, 255, 0.5);
    line-height: 1.4;
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

  .btn-text {
    position: relative;
    z-index: 1;
  }
</style>
