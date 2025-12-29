/**
 * Onboarding State Machine
 *
 * Matches Flutter beewallet onboarding flow:
 * - Create: Welcome → Reveal → Quiz → Pin → Confirm → Sealing → Identity
 * - Restore: Welcome → Restore → Pin → Confirm → Sealing → Identity
 */
import * as api from './tauri';
import { Vault } from './nine_s';

// Onboarding steps
export type OnboardingStep =
  | 'welcome'
  | 'reveal'      // Show generated mnemonic
  | 'quiz'        // Verify 3 words
  | 'restore'     // Enter 12 words
  | 'pin'         // Create PIN
  | 'confirmPin'  // Confirm PIN
  | 'sealing'     // Creating vault
  | 'identity';   // Show mobinumber

export type OnboardingFlow = 'create' | 'restore';

// State
export const onboarding = $state({
  step: 'welcome' as OnboardingStep,
  flow: null as OnboardingFlow | null,

  // Mnemonic
  mnemonic: [] as string[],
  restoreWords: Array(12).fill('') as string[],
  bip39Passphrase: '',
  showBip39Field: false,

  // Quiz
  quizIndices: [] as number[],
  quizInputs: ['', '', ''] as string[],

  // PIN
  pin: '',
  confirmPin: '',
  pinError: null as string | null,
  pinConfirmed: false,

  // Sealing
  sealError: null as string | null,

  // Identity
  mobinumber: null as string | null,

  // UI state
  loading: false,
});

// Step configuration
export function getStepConfig(flow: OnboardingFlow): { steps: OnboardingStep[]; labels: Record<OnboardingStep, number> } {
  if (flow === 'create') {
    return {
      steps: ['welcome', 'reveal', 'quiz', 'pin', 'confirmPin', 'sealing', 'identity'],
      labels: {
        welcome: 0,
        reveal: 1,
        quiz: 2,
        pin: 3,
        confirmPin: 3, // Shares step number with pin
        sealing: 4,
        identity: 5,
        restore: 0, // Not used in create
      },
    };
  } else {
    return {
      steps: ['welcome', 'restore', 'pin', 'confirmPin', 'sealing', 'identity'],
      labels: {
        welcome: 0,
        restore: 1,
        pin: 2,
        confirmPin: 2, // Shares step number with pin
        sealing: 3,
        identity: 4,
        reveal: 0, // Not used in restore
        quiz: 0,   // Not used in restore
      },
    };
  }
}

export function getCurrentStepNumber(): { current: number; total: number } {
  if (!onboarding.flow) return { current: 0, total: 0 };

  const config = getStepConfig(onboarding.flow);
  const current = config.labels[onboarding.step];
  // Total is max step number + 1 (for 0-indexing display)
  const total = onboarding.flow === 'create' ? 5 : 4;

  return { current, total };
}

// Actions
export function startCreate() {
  onboarding.flow = 'create';
  onboarding.step = 'reveal';
  generateMnemonic();
}

export function startRestore() {
  onboarding.flow = 'restore';
  onboarding.step = 'restore';
  onboarding.restoreWords = Array(12).fill('');
}

export async function generateMnemonic() {
  onboarding.loading = true;
  try {
    const phrase = await api.generateMnemonic(12);
    onboarding.mnemonic = phrase.split(' ');
    // Generate quiz indices (3 random unique indices)
    onboarding.quizIndices = generateQuizIndices();
    onboarding.quizInputs = ['', '', ''];
  } catch (e) {
    console.error('Generate mnemonic error:', e);
  } finally {
    onboarding.loading = false;
  }
}

function generateQuizIndices(): number[] {
  const indices: number[] = [];
  while (indices.length < 3) {
    const i = Math.floor(Math.random() * 12);
    if (!indices.includes(i)) indices.push(i);
  }
  return indices.sort((a, b) => a - b);
}

export function confirmReveal() {
  onboarding.step = 'quiz';
}

export function validateQuiz(): boolean {
  return onboarding.quizIndices.every((index, i) =>
    onboarding.quizInputs[i].toLowerCase().trim() ===
    onboarding.mnemonic[index].toLowerCase()
  );
}

export function confirmQuiz() {
  if (validateQuiz()) {
    onboarding.step = 'pin';
  }
}

export async function validateRestorePhrase(): Promise<boolean> {
  const phrase = onboarding.restoreWords.join(' ').trim();
  if (phrase.split(' ').length !== 12) return false;

  try {
    return await api.validateMnemonic(phrase);
  } catch {
    return false;
  }
}

export function confirmRestore() {
  onboarding.mnemonic = onboarding.restoreWords.map(w => w.trim().toLowerCase());
  onboarding.step = 'pin';
}

export function setPin(pin: string) {
  if (onboarding.step === 'pin') {
    onboarding.pin = pin;
    onboarding.step = 'confirmPin';
    onboarding.pinError = null;
  } else if (onboarding.step === 'confirmPin') {
    if (pin === onboarding.pin) {
      onboarding.confirmPin = pin;
      onboarding.pinConfirmed = true;
      // Brief animation delay then seal
      setTimeout(() => seal(), 800);
    } else {
      onboarding.pinError = 'PINs do not match';
      onboarding.confirmPin = '';
    }
  }
}

export function backToPin() {
  onboarding.step = 'pin';
  onboarding.pin = '';
  onboarding.confirmPin = '';
  onboarding.pinError = null;
  onboarding.pinConfirmed = false;
}

export async function seal() {
  onboarding.step = 'sealing';
  onboarding.sealError = null;
  onboarding.loading = true;

  try {
    const phrase = onboarding.mnemonic.join(' ');
    const passphrase = onboarding.bip39Passphrase || undefined;

    // Get mobinumber before connecting (so we have it even if connect times out)
    const mobinumber = await api.getMobinumber(phrase);
    onboarding.mobinumber = mobinumber;

    // Initialize vault with PIN and mnemonic (encrypts the seed)
    await Vault.init(onboarding.pin, phrase);

    // Connect to wallet with mnemonic
    await api.connect(phrase, passphrase, 'regtest');

    onboarding.step = 'identity';
  } catch (e: any) {
    onboarding.sealError = e?.message || String(e);
  } finally {
    onboarding.loading = false;
  }
}

export function retry() {
  seal();
}

export function reset() {
  onboarding.step = 'welcome';
  onboarding.flow = null;
  onboarding.mnemonic = [];
  onboarding.restoreWords = Array(12).fill('');
  onboarding.bip39Passphrase = '';
  onboarding.showBip39Field = false;
  onboarding.quizIndices = [];
  onboarding.quizInputs = ['', '', ''];
  onboarding.pin = '';
  onboarding.confirmPin = '';
  onboarding.pinError = null;
  onboarding.pinConfirmed = false;
  onboarding.sealError = null;
  onboarding.mobinumber = null;
  onboarding.loading = false;
}

// BIP39 word validation (basic check - full validation in Rust)
const BIP39_WORDS = new Set([
  'abandon', 'ability', 'able', 'about', 'above', 'absent', 'absorb', 'abstract',
  'absurd', 'abuse', 'access', 'accident', 'account', 'accuse', 'achieve', 'acid',
  // ... This would normally be the full 2048-word list
  // For now, just basic validation that words exist
]);

export function isValidBip39Word(word: string): boolean {
  // In production, this calls Rust: rust.isValidBip39Word(word)
  // For now, accept any lowercase word with 3+ chars
  return word.length >= 3 && /^[a-z]+$/.test(word);
}

export function getBip39Suggestions(prefix: string, limit = 4): string[] {
  // In production, this calls Rust: rust.bip39Suggestions(prefix, limit)
  // Placeholder - returns empty array
  return [];
}
