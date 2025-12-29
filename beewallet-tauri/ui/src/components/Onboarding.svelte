<script lang="ts">
  import { onMount } from 'svelte';
  import {
    onboarding,
    getCurrentStepNumber,
    startCreate,
    startRestore,
    confirmReveal,
    confirmQuiz,
    validateQuiz,
    confirmRestore,
    setPin,
    backToPin,
    retry,
    reset,
  } from '../lib/onboarding.svelte';

  import StepIndicator from './StepIndicator.svelte';
  import Welcome from './onboarding/Welcome.svelte';
  import MnemonicReveal from './onboarding/MnemonicReveal.svelte';
  import Quiz from './onboarding/Quiz.svelte';
  import RestoreGrid from './onboarding/RestoreGrid.svelte';
  import PinEntry from './onboarding/PinEntry.svelte';
  import Sealing from './onboarding/Sealing.svelte';
  import Identity from './onboarding/Identity.svelte';

  interface Props {
    onComplete: () => void;
    startInRestore?: boolean;
  }

  let { onComplete, startInRestore = false }: Props = $props();

  onMount(() => {
    // If starting in restore mode (e.g., from migration), go directly to restore
    if (startInRestore && onboarding.step === 'welcome') {
      startRestore();
    }
  });
</script>

<div class="onboarding">
  {#if onboarding.flow && onboarding.step !== 'welcome'}
    {@const stepInfo = getCurrentStepNumber()}
    <StepIndicator current={stepInfo.current} total={stepInfo.total} />
  {/if}

  <div class="content">
    {#if onboarding.step === 'welcome'}
      <Welcome onCreate={startCreate} onRestore={startRestore} />
    {:else if onboarding.step === 'reveal'}
      <MnemonicReveal
        mnemonic={onboarding.mnemonic}
        onConfirm={confirmReveal}
        onBack={() => reset()}
      />
    {:else if onboarding.step === 'quiz'}
      <Quiz
        mnemonic={onboarding.mnemonic}
        indices={onboarding.quizIndices}
        bind:inputs={onboarding.quizInputs}
        onConfirm={confirmQuiz}
        onBack={() => onboarding.step = 'reveal'}
        isValid={validateQuiz()}
      />
    {:else if onboarding.step === 'restore'}
      <RestoreGrid
        bind:words={onboarding.restoreWords}
        bind:passphrase={onboarding.bip39Passphrase}
        bind:showPassphrase={onboarding.showBip39Field}
        onConfirm={confirmRestore}
        onBack={() => reset()}
      />
    {:else if onboarding.step === 'pin'}
      <PinEntry
        title="Create a PIN"
        subtitle="Enter a 6-digit PIN to protect your wallet"
        step={1}
        onComplete={setPin}
        onBack={() => onboarding.flow === 'create' ? onboarding.step = 'quiz' : reset()}
      />
    {:else if onboarding.step === 'confirmPin'}
      <PinEntry
        title="Confirm PIN"
        subtitle="Re-enter your 6-digit PIN"
        step={2}
        error={onboarding.pinError}
        showSuccess={onboarding.pinConfirmed}
        onComplete={setPin}
        onBack={backToPin}
      />
    {:else if onboarding.step === 'sealing'}
      <Sealing
        error={onboarding.sealError}
        onRetry={retry}
        onBack={backToPin}
        onStartOver={reset}
      />
    {:else if onboarding.step === 'identity'}
      <Identity
        mobinumber={onboarding.mobinumber || ''}
        onEnter={onComplete}
      />
    {/if}
  </div>
</div>

<style>
  .onboarding {
    min-height: 100vh;
    padding: var(--spacing-df);
    display: flex;
    flex-direction: column;
  }

  .content {
    flex: 1;
    display: flex;
    flex-direction: column;
  }
</style>
