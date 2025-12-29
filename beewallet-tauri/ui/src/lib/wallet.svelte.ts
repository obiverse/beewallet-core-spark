/**
 * Reactive wallet store using Svelte 5 runes
 */
import * as api from './tauri';

// Wallet state
export const wallet = $state({
  connected: false,
  connecting: false,
  syncing: false,
  balance: 0,
  address: '',
  payments: [] as Payment[],
  error: null as string | null,
});

export interface Payment {
  id: string;
  type: 'send' | 'receive';
  amount: number;
  status: string;
  timestamp?: number;
}

// Check connection on load
export async function checkConnection(): Promise<boolean> {
  try {
    wallet.connected = await api.isConnected();
    if (wallet.connected) {
      await refresh();
    }
    return wallet.connected;
  } catch (e) {
    console.error('Check connection error:', e);
    return false;
  }
}

// Connect wallet
export async function connect(mnemonic: string, passphrase?: string) {
  wallet.connecting = true;
  wallet.error = null;

  try {
    await api.connect(mnemonic, passphrase, 'regtest');
    wallet.connected = true;
    await refresh();
  } catch (e: any) {
    wallet.error = e?.message || String(e);
    throw e;
  } finally {
    wallet.connecting = false;
  }
}

// Disconnect wallet
export async function disconnect() {
  try {
    await api.disconnect();
    wallet.connected = false;
    wallet.balance = 0;
    wallet.address = '';
    wallet.payments = [];
  } catch (e: any) {
    wallet.error = e?.message || String(e);
  }
}

// Refresh wallet data
export async function refresh() {
  try {
    wallet.balance = await api.getBalance();
    wallet.address = await api.getAddress();
    await refreshPayments();
  } catch (e: any) {
    console.error('Refresh error:', e);
    // Don't set error for refresh failures
  }
}

// Refresh payments
async function refreshPayments() {
  try {
    const result = await api.listPayments(20);
    wallet.payments = (result?.payments as Payment[]) || [];
  } catch (e) {
    console.error('Refresh payments error:', e);
  }
}

// Generate mnemonic
export async function generateMnemonic(): Promise<string> {
  return api.generateMnemonic(12);
}

// Create invoice
export async function createInvoice(amountSat: number, description?: string) {
  const result = await api.createInvoice(amountSat, description);
  return result;
}

// Send payment
export async function sendPayment(destination: string, amountSat?: number) {
  const result = await api.sendPayment(destination, amountSat);
  await refresh();
  return result;
}

// Sync wallet
export async function sync() {
  wallet.syncing = true;
  try {
    await api.syncWallet();
    await refresh();
  } finally {
    wallet.syncing = false;
  }
}

// Alias for Wallet.svelte
export const syncWallet = sync;
export const refreshBalance = refresh;

// Format helpers
export function formatSats(sats: number): string {
  return new Intl.NumberFormat().format(sats);
}

export function formatBtc(sats: number): string {
  return (sats / 100_000_000).toFixed(8);
}
