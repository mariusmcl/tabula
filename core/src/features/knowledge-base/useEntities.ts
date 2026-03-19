import { useState, useCallback } from 'react';
import type { Entity } from '../../types';
import { listEntities, queryKnowledgeBase, checkConnection } from '../../services/tabula-kb';

interface UseEntitiesReturn {
  entities: Entity[];
  isLoading: boolean;
  error: string | null;
  loadEntities: (entityType: string) => Promise<void>;
  searchEntities: (query: string) => Promise<void>;
  isConnected: boolean;
  checkNodeConnection: () => Promise<void>;
}

export function useEntities(): UseEntitiesReturn {
  const [entities, setEntities] = useState<Entity[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isConnected, setIsConnected] = useState(false);

  const checkNodeConnection = useCallback(async () => {
    const connected = await checkConnection();
    setIsConnected(connected);
  }, []);

  const loadEntities = useCallback(async (entityType: string) => {
    setIsLoading(true);
    setError(null);

    try {
      const result = await listEntities(entityType);
      setEntities(result);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load entities';
      setError(message);
      setEntities([]);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const searchEntities = useCallback(async (query: string) => {
    setIsLoading(true);
    setError(null);

    try {
      const result = await queryKnowledgeBase(query);
      setEntities(result.entities);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to search entities';
      setError(message);
      setEntities([]);
    } finally {
      setIsLoading(false);
    }
  }, []);

  return {
    entities,
    isLoading,
    error,
    loadEntities,
    searchEntities,
    isConnected,
    checkNodeConnection,
  };
}
