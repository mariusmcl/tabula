import { useState, useEffect, useCallback } from 'react';
import type { MiningProgress } from '../../types';
import {
  startMining,
  stopMining,
  getMiningStatus,
} from '../../services/blockchain';
import { subscribeMiningProgress, subscribeBlockMined } from '../../services/events';

interface UseMiningReturn {
  isMining: boolean;
  progress: MiningProgress | null;
  hashRate: number;
  elapsedTime: number;
  start: (data: string) => Promise<void>;
  stop: () => Promise<void>;
  error: string | null;
}

export function useMining(): UseMiningReturn {
  const [isMining, setIsMining] = useState(false);
  const [progress, setProgress] = useState<MiningProgress | null>(null);
  const [hashRate, setHashRate] = useState(0);
  const [elapsedTime, setElapsedTime] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const start = useCallback(async (data: string) => {
    setError(null);
    setIsMining(true);
    setProgress(null);
    setHashRate(0);
    setElapsedTime(0);

    try {
      await startMining(data);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to start mining';
      setError(message);
      setIsMining(false);
    }
  }, []);

  const stop = useCallback(async () => {
    try {
      await stopMining();
      setIsMining(false);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to stop mining';
      setError(message);
    }
  }, []);

  // Subscribe to mining progress events
  useEffect(() => {
    const unsubscribe = subscribeMiningProgress((event) => {
      setProgress(event);
      setHashRate(event.hashRate);
      setElapsedTime(event.elapsed);
    });

    return unsubscribe;
  }, []);

  // Subscribe to block mined events
  useEffect(() => {
    const unsubscribe = subscribeBlockMined(() => {
      setIsMining(false);
      setProgress(null);
    });

    return unsubscribe;
  }, []);

  // Poll mining status on mount
  useEffect(() => {
    const checkStatus = async () => {
      try {
        const status = await getMiningStatus();
        setIsMining(status.is_mining);
        if (status.is_mining) {
          setHashRate(status.hash_rate);
          setElapsedTime(status.elapsed_ms);
        }
      } catch {
        // Ignore errors - command might not be implemented yet
      }
    };

    checkStatus();
  }, []);

  return {
    isMining,
    progress,
    hashRate,
    elapsedTime,
    start,
    stop,
    error,
  };
}
