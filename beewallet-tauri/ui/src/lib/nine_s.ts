/**
 * 9S Bus Client
 *
 * Everything flows through this single interface:
 *
 * ```typescript
 * // Read data
 * const status = await NineS.read('/system/status');
 * console.log(status.data.connected);
 *
 * // Write data
 * const result = await NineS.write('/wallet/send', { destination, amount_sat });
 *
 * // List paths
 * const paths = await NineS.list('/wallet');
 *
 * // Listen for events
 * NineS.on('nine_s://wallet/payment', (event) => {
 *   console.log('New payment:', event);
 * });
 * ```
 *
 * ## Path Ontology
 *
 * | Path | Op | Description |
 * |------|-----|-------------|
 * | `/system/info` | read | App name, version, network |
 * | `/system/status` | read | Connection status, wallet exists |
 * | `/system/connect` | write | Connect with mnemonic |
 * | `/system/disconnect` | write | Disconnect wallet |
 * | `/identity/mnemonic` | write | Generate new mnemonic |
 * | `/identity/validate` | write | Validate mnemonic phrase |
 * | `/identity/mobinumber` | write | Derive mobinumber from phrase |
 * | `/wallet/balance` | read | Get balance in sats |
 * | `/wallet/address` | read | Get receive address |
 * | `/wallet/send` | write | Send payment |
 * | `/wallet/invoice` | write | Create invoice |
 * | `/wallet/payments` | read | List payments |
 * | `/wallet/sync` | write | Sync with network |
 */

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// ============================================================================
// TYPES
// ============================================================================

/** Scroll data from backend */
export interface Scroll {
  key: string;
  data: Record<string, unknown>;
  type: string;
}

/** 9S response from backend */
interface NineSResponse {
  ok: boolean;
  scroll?: Scroll;
  paths?: string[];
  error?: string;
}

/** 9S request to backend */
interface NineSRequest {
  op: 'read' | 'write' | 'list';
  path: string;
  data?: Record<string, unknown>;
}

/** Event callback type */
type EventCallback = (data: unknown) => void;

// ============================================================================
// 9S BUS CLIENT
// ============================================================================

class NineSClient {
  private listeners: Map<string, UnlistenFn[]> = new Map();

  /**
   * Read from a path
   *
   * @example
   * const status = await NineS.read('/system/status');
   * console.log(status?.data.connected);
   */
  async read(path: string): Promise<Scroll | null> {
    const request: NineSRequest = { op: 'read', path };
    const response = await invoke<NineSResponse>('nine_s', { request });

    if (!response.ok) {
      throw new NineSError(path, response.error || 'Unknown error');
    }

    return response.scroll || null;
  }

  /**
   * Write to a path
   *
   * @example
   * const result = await NineS.write('/wallet/send', {
   *   destination: 'bolt11...',
   *   amount_sat: 1000
   * });
   */
  async write(path: string, data: Record<string, unknown> = {}): Promise<Scroll> {
    const request: NineSRequest = { op: 'write', path, data };
    const response = await invoke<NineSResponse>('nine_s', { request });

    if (!response.ok) {
      throw new NineSError(path, response.error || 'Unknown error');
    }

    if (!response.scroll) {
      throw new NineSError(path, 'No scroll returned');
    }

    return response.scroll;
  }

  /**
   * List paths under a prefix
   *
   * @example
   * const paths = await NineS.list('/wallet');
   * // ['/wallet/balance', '/wallet/address', ...]
   */
  async list(prefix: string = '/'): Promise<string[]> {
    const request: NineSRequest = { op: 'list', path: prefix };
    const response = await invoke<NineSResponse>('nine_s', { request });

    if (!response.ok) {
      throw new NineSError(prefix, response.error || 'Unknown error');
    }

    return response.paths || [];
  }

  /**
   * Listen for 9S events from backend
   *
   * @example
   * const unlisten = await NineS.on('nine_s://wallet/payment', (event) => {
   *   console.log('New payment:', event);
   * });
   *
   * // Later: unlisten();
   */
  async on(event: string, callback: EventCallback): Promise<UnlistenFn> {
    const unlisten = await listen(event, (e) => callback(e.payload));

    // Track for cleanup
    const existing = this.listeners.get(event) || [];
    existing.push(unlisten);
    this.listeners.set(event, existing);

    return unlisten;
  }

  /**
   * Remove all listeners for an event
   */
  off(event: string): void {
    const listeners = this.listeners.get(event) || [];
    listeners.forEach(unlisten => unlisten());
    this.listeners.delete(event);
  }

  /**
   * Remove all listeners
   */
  offAll(): void {
    this.listeners.forEach((listeners) => {
      listeners.forEach(unlisten => unlisten());
    });
    this.listeners.clear();
  }
}

// ============================================================================
// ERROR TYPE
// ============================================================================

export class NineSError extends Error {
  constructor(
    public path: string,
    message: string
  ) {
    super(`9S error at ${path}: ${message}`);
    this.name = 'NineSError';
  }
}

// ============================================================================
// CONVENIENCE HELPERS
// ============================================================================

/**
 * Typed read helper
 *
 * @example
 * const balance = await read<{ balance_sat: number }>('/wallet/balance');
 * console.log(balance?.balance_sat);
 */
export async function read<T = Record<string, unknown>>(path: string): Promise<T | null> {
  const scroll = await NineS.read(path);
  return scroll?.data as T | null;
}

/**
 * Typed write helper
 *
 * @example
 * const result = await write<{ txid: string }>('/wallet/send', { ... });
 * console.log(result.txid);
 */
export async function write<T = Record<string, unknown>>(
  path: string,
  data: Record<string, unknown> = {}
): Promise<T> {
  const scroll = await NineS.write(path, data);
  return scroll.data as T;
}

// ============================================================================
// SINGLETON EXPORT
// ============================================================================

export const NineS = new NineSClient();
export default NineS;

// ============================================================================
// NAMESPACE HELPERS (optional ergonomic layer)
// ============================================================================

/** System namespace operations */
export const System = {
  async info() {
    return read<{
      name: string;
      version: string;
      backend: string;
      network: string;
      faucet: string;
    }>('/system/info');
  },

  async status() {
    return read<{
      connected: boolean;
      wallet_exists: boolean;
      working_dir: string;
    }>('/system/status');
  },

  async connect(mnemonic: string, passphrase?: string, network = 'regtest') {
    return write<{ status: string; network: string }>('/system/connect', {
      mnemonic,
      passphrase,
      network,
    });
  },

  async disconnect() {
    return write<{ status: string }>('/system/disconnect', {});
  },
};

/** Identity namespace operations */
export const Identity = {
  async generateMnemonic(wordCount = 12) {
    return write<{ phrase: string; word_count: number }>('/identity/mnemonic', {
      word_count: wordCount,
    });
  },

  async validate(phrase: string) {
    const result = await write<{ valid: boolean }>('/identity/validate', { phrase });
    return result.valid;
  },

  async mobinumber(phrase: string) {
    const result = await write<{ mobinumber: string }>('/identity/mobinumber', { phrase });
    return result.mobinumber;
  },
};

/** Vault namespace operations - secure mnemonic storage */
export const Vault = {
  /** Get vault status */
  async status() {
    return read<{
      initialized: boolean;
      unlocked: boolean;
      lockout_remaining: number;
    }>('/vault/status');
  },

  /** Initialize vault with PIN and mnemonic */
  async init(pin: string, mnemonic: string) {
    return write<{ status: string }>('/vault/init', { pin, mnemonic });
  },

  /** Unlock vault with PIN */
  async unlock(pin: string) {
    return write<{ status: string }>('/vault/unlock', { pin });
  },

  /** Lock the vault */
  async lock() {
    return write<{ status: string }>('/vault/lock', {});
  },

  /** Reset vault (DANGER: destroys encrypted seed) */
  async reset() {
    return write<{ status: string }>('/vault/reset', {});
  },

  /** Auto-connect wallet after vault unlock */
  async autoConnect(network = 'regtest') {
    return write<{ status: string; network: string }>('/vault/auto-connect', { network });
  },
};

/** Wallet namespace operations */
export const Wallet = {
  async balance() {
    const result = await read<{ balance_sat: number }>('/wallet/balance');
    return result?.balance_sat ?? 0;
  },

  /** Get Spark address for Lightning/Spark payments */
  async address() {
    const result = await read<{ address: string }>('/wallet/address');
    return result?.address ?? '';
  },

  /** Get Bitcoin address for on-chain deposits (faucet, exchanges) */
  async bitcoinAddress() {
    const result = await read<{ address: string; fee_sat: number }>('/wallet/bitcoin-address');
    return result ?? { address: '', fee_sat: 0 };
  },

  async send(destination: string, amount_sat?: number) {
    return write('/wallet/send', { destination, amount_sat });
  },

  async invoice(amount_sat: number, description?: string) {
    return write<{ invoice: string; bolt11?: string }>('/wallet/invoice', {
      amount_sat,
      description,
    });
  },

  async payments() {
    return read<{ payments: unknown[] }>('/wallet/payments');
  },

  async sync() {
    return write('/wallet/sync', {});
  },
};
