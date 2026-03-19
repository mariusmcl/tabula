import { useState, useEffect, useCallback } from 'react';
import type { Block, AsyncState } from '../../types';
import {
  getBlockchain,
  addBlock,
  validateChain,
  getDifficulty,
} from '../../services/blockchain';
import { subscribeChainUpdated } from '../../services/events';

interface UseBlockchainReturn {
  blocks: Block[];
  difficulty: number;
  isValid: boolean;
  status: AsyncState<Block[]>['status'];
  error: string | null;
  refresh: () => Promise<void>;
  mineBlock: (data: string) => Promise<void>;
  isMining: boolean;
}

export function useBlockchain(): UseBlockchainReturn {
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [difficulty, setDifficulty] = useState<number>(2);
  const [isValid, setIsValid] = useState<boolean>(true);
  const [status, setStatus] = useState<AsyncState<Block[]>['status']>('idle');
  const [error, setError] = useState<string | null>(null);
  const [isMining, setIsMining] = useState<boolean>(false);

  const refresh = useCallback(async () => {
    setStatus('loading');
    setError(null);

    try {
      const [chain, diff, valid] = await Promise.all([
        getBlockchain(),
        getDifficulty(),
        validateChain(),
      ]);

      setBlocks(chain);
      setDifficulty(diff);
      setIsValid(valid);
      setStatus('success');
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load blockchain';
      setError(message);
      setStatus('error');
    }
  }, []);

  const mineBlock = useCallback(async (data: string) => {
    if (isMining) return;

    setIsMining(true);
    setError(null);

    try {
      const updatedChain = await addBlock(data);
      setBlocks(updatedChain);
      setIsValid(true);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to mine block';
      setError(message);
    } finally {
      setIsMining(false);
    }
  }, [isMining]);

  // Load blockchain on mount
  useEffect(() => {
    refresh();
  }, [refresh]);

  // Subscribe to chain updates
  useEffect(() => {
    const unsubscribe = subscribeChainUpdated((updatedChain) => {
      setBlocks(updatedChain);
    });

    return unsubscribe;
  }, []);

  return {
    blocks,
    difficulty,
    isValid,
    status,
    error,
    refresh,
    mineBlock,
    isMining,
  };
}
