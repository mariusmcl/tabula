import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Block, MiningProgressEvent } from '../types';

type EventCallback<T> = (payload: T) => void;

interface EventHub<T> {
  subscribe: (callback: EventCallback<T>) => () => void;
}

/**
 * Create an event hub for a specific Tauri event
 */
function createEventHub<T>(eventName: string): EventHub<T> {
  const listeners = new Set<EventCallback<T>>();
  let unlisten: UnlistenFn | null = null;
  let isListening = false;

  const startListening = async () => {
    if (isListening) return;
    isListening = true;

    try {
      unlisten = await listen<T>(eventName, (event) => {
        for (const listener of listeners) {
          try {
            listener(event.payload);
          } catch (error) {
            console.error(`[events] ${eventName} listener failed:`, error);
          }
        }
      });
    } catch (error) {
      console.warn(`[events] Failed to listen for ${eventName}:`, error);
      isListening = false;
    }
  };

  const stopListening = () => {
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
    isListening = false;
  };

  return {
    subscribe: (callback: EventCallback<T>) => {
      listeners.add(callback);

      // Start listening when first subscriber added
      if (listeners.size === 1) {
        startListening();
      }

      // Return unsubscribe function
      return () => {
        listeners.delete(callback);

        // Stop listening when last subscriber removed
        if (listeners.size === 0) {
          stopListening();
        }
      };
    },
  };
}

// Mining progress event hub
const miningProgressHub = createEventHub<MiningProgressEvent>('mining:progress');

/**
 * Subscribe to mining progress events
 */
export function subscribeMiningProgress(
  callback: EventCallback<MiningProgressEvent>
): () => void {
  return miningProgressHub.subscribe(callback);
}

// Block mined event hub
const blockMinedHub = createEventHub<Block>('block:mined');

/**
 * Subscribe to block mined events
 */
export function subscribeBlockMined(callback: EventCallback<Block>): () => void {
  return blockMinedHub.subscribe(callback);
}

// Chain updated event hub
const chainUpdatedHub = createEventHub<Block[]>('chain:updated');

/**
 * Subscribe to chain update events
 */
export function subscribeChainUpdated(
  callback: EventCallback<Block[]>
): () => void {
  return chainUpdatedHub.subscribe(callback);
}

// Mining error event hub
const miningErrorHub = createEventHub<string>('mining:error');

/**
 * Subscribe to mining error events
 */
export function subscribeMiningError(callback: EventCallback<string>): () => void {
  return miningErrorHub.subscribe(callback);
}
