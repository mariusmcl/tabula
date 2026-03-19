import { useState, useEffect, useCallback } from 'react';
import type { NodeStatus, PeerInfo, ChainInfo } from '../../types';
import {
  getNodeStatus,
  getPeers,
  getChainInfo,
  checkConnection,
  getMockNodeStatus,
  getMockPeers,
  getMockChainInfo,
} from '../../services/tabula-kb';

interface UseNetworkReturn {
  isConnected: boolean;
  nodeStatus: NodeStatus | null;
  peers: PeerInfo[];
  chainInfo: ChainInfo | null;
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  connect: () => Promise<void>;
}

export function useNetwork(): UseNetworkReturn {
  const [isConnected, setIsConnected] = useState(false);
  const [nodeStatus, setNodeStatus] = useState<NodeStatus | null>(null);
  const [peers, setPeers] = useState<PeerInfo[]>([]);
  const [chainInfo, setChainInfo] = useState<ChainInfo | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const connected = await checkConnection();
      setIsConnected(connected);

      if (connected) {
        const [status, peerList, chain] = await Promise.all([
          getNodeStatus(),
          getPeers(),
          getChainInfo(),
        ]);

        setNodeStatus(status);
        setPeers(peerList);
        setChainInfo(chain);
      } else {
        // Use mock data when not connected
        setNodeStatus(getMockNodeStatus());
        setPeers(getMockPeers());
        setChainInfo(getMockChainInfo());
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to fetch network data';
      setError(message);
      setIsConnected(false);

      // Fall back to mock data on error
      setNodeStatus(getMockNodeStatus());
      setPeers(getMockPeers());
      setChainInfo(getMockChainInfo());
    } finally {
      setIsLoading(false);
    }
  }, []);

  const connect = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const connected = await checkConnection();
      setIsConnected(connected);

      if (connected) {
        await refresh();
      } else {
        setError('Unable to connect to tabula-kb node. Make sure the node is running.');
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Connection failed';
      setError(message);
      setIsConnected(false);
    } finally {
      setIsLoading(false);
    }
  }, [refresh]);

  // Initial load
  useEffect(() => {
    refresh();
  }, [refresh]);

  // Periodic refresh when connected
  useEffect(() => {
    if (!isConnected) return;

    const interval = setInterval(refresh, 10000); // Refresh every 10s
    return () => clearInterval(interval);
  }, [isConnected, refresh]);

  return {
    isConnected,
    nodeStatus,
    peers,
    chainInfo,
    isLoading,
    error,
    refresh,
    connect,
  };
}
