import { invoke } from '@tauri-apps/api/core';
import type { Block, MiningStatus } from '../types';

/**
 * Check if we're running in a Tauri environment
 */
function isTauriAvailable(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

/**
 * Get the current blockchain state
 */
export async function getBlockchain(): Promise<Block[]> {
  if (!isTauriAvailable()) {
    console.warn('Tauri not available, returning mock data');
    return getMockBlockchain();
  }
  return invoke<Block[]>('get_blockchain');
}

/**
 * Add a new block to the blockchain (triggers mining)
 */
export async function addBlock(data: string): Promise<Block[]> {
  if (!isTauriAvailable()) {
    console.warn('Tauri not available, returning mock data');
    return getMockBlockchain();
  }
  return invoke<Block[]>('add_block', { data });
}

/**
 * Validate the entire blockchain
 */
export async function validateChain(): Promise<boolean> {
  if (!isTauriAvailable()) {
    return true;
  }
  return invoke<boolean>('validate_chain');
}

/**
 * Get the current mining difficulty
 */
export async function getDifficulty(): Promise<number> {
  if (!isTauriAvailable()) {
    return 2;
  }
  return invoke<number>('get_difficulty');
}

/**
 * Start mining a new block (async mining - to be implemented in backend)
 */
export async function startMining(data: string): Promise<void> {
  if (!isTauriAvailable()) {
    console.warn('Tauri not available');
    return;
  }
  return invoke<void>('start_mining', { data });
}

/**
 * Stop the current mining operation
 */
export async function stopMining(): Promise<void> {
  if (!isTauriAvailable()) {
    return;
  }
  return invoke<void>('stop_mining');
}

/**
 * Get current mining status
 */
export async function getMiningStatus(): Promise<MiningStatus> {
  if (!isTauriAvailable()) {
    return {
      is_mining: false,
      current_nonce: 0,
      hash_rate: 0,
      elapsed_ms: 0,
      target_difficulty: 2,
    };
  }
  return invoke<MiningStatus>('get_mining_status');
}

/**
 * Mock blockchain for development without Tauri
 */
function getMockBlockchain(): Block[] {
  const now = Math.floor(Date.now() / 1000);
  return [
    {
      index: 0,
      timestamp: now - 3600,
      data: 'Genesis Block',
      previous_hash: '0',
      hash: '00a1b2c3d4e5f6789012345678901234567890123456789012345678901234567',
      nonce: 0,
    },
    {
      index: 1,
      timestamp: now - 1800,
      data: 'First transaction',
      previous_hash: '00a1b2c3d4e5f6789012345678901234567890123456789012345678901234567',
      hash: '00b2c3d4e5f67890123456789012345678901234567890123456789012345678',
      nonce: 1234,
    },
    {
      index: 2,
      timestamp: now - 600,
      data: 'Second transaction',
      previous_hash: '00b2c3d4e5f67890123456789012345678901234567890123456789012345678',
      hash: '00c3d4e5f678901234567890123456789012345678901234567890123456789',
      nonce: 5678,
    },
  ];
}

/**
 * Format a timestamp to a readable date string
 */
export function formatTimestamp(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  return date.toLocaleString();
}

/**
 * Truncate a hash for display
 */
export function truncateHash(hash: string, chars: number = 8): string {
  if (hash.length <= chars * 2) return hash;
  return `${hash.slice(0, chars)}...${hash.slice(-chars)}`;
}

/**
 * Format nonce with thousands separator
 */
export function formatNonce(nonce: number): string {
  return nonce.toLocaleString();
}

/**
 * Calculate hash rate from nonce and elapsed time
 */
export function calculateHashRate(nonce: number, elapsedMs: number): number {
  if (elapsedMs === 0) return 0;
  return Math.round((nonce / elapsedMs) * 1000);
}
