/**
 * Tauri API - Legacy wrapper over 9S bus
 *
 * @deprecated Use `import { NineS, System, Identity, Wallet } from './nine_s'` instead
 *
 * This file provides backwards compatibility while migrating to the 9S pattern.
 */

import { System, Identity, Vault, Wallet, NineS } from './nine_s';

// Re-export 9S types
export type { Scroll, NineSError } from './nine_s';
export { NineS, System, Identity, Vault, Wallet };

// ============================================================================
// LEGACY API (for backwards compat during migration)
// ============================================================================

/** @deprecated Use System.info() */
export async function systemInfo() {
  return System.info();
}

/** @deprecated Use System.status() */
export async function isConnected(): Promise<boolean> {
  const status = await System.status();
  return status?.connected ?? false;
}

/** @deprecated Use System.status() */
export async function walletExists(): Promise<boolean> {
  const status = await System.status();
  return status?.wallet_exists ?? false;
}

/** @deprecated Use System.status() */
export async function getWorkingDir(): Promise<string> {
  const status = await System.status();
  return status?.working_dir ?? '';
}

/** @deprecated Use System.connect() */
export async function connect(
  mnemonic: string,
  passphrase?: string,
  network?: string
): Promise<{ status: string; network: string }> {
  return System.connect(mnemonic, passphrase, network);
}

/** @deprecated Use System.disconnect() */
export async function disconnect(): Promise<{ status: string }> {
  return System.disconnect();
}

/** @deprecated Use Identity.generateMnemonic() */
export async function generateMnemonic(wordCount?: number): Promise<string> {
  const result = await Identity.generateMnemonic(wordCount);
  return result.phrase;
}

/** @deprecated Use Identity.validate() */
export async function validateMnemonic(phrase: string): Promise<boolean> {
  return Identity.validate(phrase);
}

/** @deprecated Use Identity.mobinumber() */
export async function getMobinumber(phrase: string): Promise<string> {
  return Identity.mobinumber(phrase);
}

/** @deprecated Use Wallet.balance() */
export async function getBalance(): Promise<number> {
  return Wallet.balance();
}

/** @deprecated Use Wallet.address() */
export async function getAddress(): Promise<string> {
  return Wallet.address();
}

/** @deprecated Use Wallet.invoice() */
export async function createInvoice(
  amountSat: number,
  description?: string
): Promise<Record<string, unknown>> {
  return Wallet.invoice(amountSat, description);
}

/** @deprecated Use Wallet.send() */
export async function sendPayment(
  destination: string,
  amountSat?: number
): Promise<Record<string, unknown>> {
  return Wallet.send(destination, amountSat);
}

/** @deprecated Use Wallet.payments() */
export async function listPayments(limit?: number): Promise<Record<string, unknown>> {
  // Note: limit not currently supported in 9S, would need to add
  const result = await Wallet.payments();
  return result ?? { payments: [] };
}

/** @deprecated Use Wallet.sync() */
export async function syncWallet(): Promise<Record<string, unknown>> {
  return Wallet.sync();
}

// ============================================================================
// 9S PROTOCOL DIRECT ACCESS
// ============================================================================

/** @deprecated Use NineS.read() */
export async function read(path: string) {
  return NineS.read(path);
}

/** @deprecated Use NineS.write() */
export async function write(path: string, data: Record<string, unknown>) {
  return NineS.write(path, data);
}

/** @deprecated Use NineS.list() */
export async function list(prefix: string): Promise<string[]> {
  return NineS.list(prefix);
}
