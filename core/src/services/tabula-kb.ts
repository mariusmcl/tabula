import type { Entity, QueryResult, NodeStatus, PeerInfo, ChainInfo } from '../types';

/**
 * Service for connecting to tabula-kb node via HTTP API
 * The tabula-kb node runs as a separate process
 */

const DEFAULT_NODE_URL = 'http://localhost:8080';

let nodeUrl = DEFAULT_NODE_URL;

/**
 * Configure the tabula-kb node URL
 */
export function setNodeUrl(url: string): void {
  nodeUrl = url;
}

/**
 * Get the current node URL
 */
export function getNodeUrl(): string {
  return nodeUrl;
}

/**
 * Make an API request to the tabula-kb node
 */
async function apiRequest<T>(
  endpoint: string,
  options?: RequestInit
): Promise<T> {
  const url = `${nodeUrl}/api${endpoint}`;

  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`API request failed: ${response.status} - ${error}`);
  }

  return response.json();
}

/**
 * Get the node status
 */
export async function getNodeStatus(): Promise<NodeStatus> {
  return apiRequest<NodeStatus>('/status');
}

/**
 * Get list of connected peers
 */
export async function getPeers(): Promise<PeerInfo[]> {
  return apiRequest<PeerInfo[]>('/peers');
}

/**
 * Get chain information
 */
export async function getChainInfo(): Promise<ChainInfo> {
  return apiRequest<ChainInfo>('/chain');
}

/**
 * Query the knowledge base
 */
export async function queryKnowledgeBase(query: string): Promise<QueryResult> {
  return apiRequest<QueryResult>('/kb/query', {
    method: 'POST',
    body: JSON.stringify({ query }),
  });
}

/**
 * List entities by type
 */
export async function listEntities(entityType: string): Promise<Entity[]> {
  return apiRequest<Entity[]>(`/kb/entities/${encodeURIComponent(entityType)}`);
}

/**
 * Check if the tabula-kb node is reachable
 */
export async function checkConnection(): Promise<boolean> {
  try {
    await getNodeStatus();
    return true;
  } catch {
    return false;
  }
}

/**
 * Mock data for development when node is not available
 */
export function getMockNodeStatus(): NodeStatus {
  return {
    node_id: 'local-dev-node',
    is_running: false,
    peers_count: 0,
    chain_height: 0,
    synced: false,
  };
}

export function getMockPeers(): PeerInfo[] {
  return [
    {
      id: 'peer-1',
      address: '192.168.1.100:8080',
      status: 'connected',
      latency_ms: 45,
      last_seen: Date.now(),
    },
    {
      id: 'peer-2',
      address: '192.168.1.101:8080',
      status: 'syncing',
      latency_ms: 120,
      last_seen: Date.now() - 5000,
    },
  ];
}

export function getMockChainInfo(): ChainInfo {
  return {
    height: 1000,
    difficulty: 4,
    total_transactions: 5432,
    last_block_time: Date.now() - 60000,
  };
}
