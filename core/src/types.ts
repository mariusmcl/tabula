// Block types matching Rust backend
export interface Block {
  index: number;
  timestamp: number;
  data: string;
  previous_hash: string;
  hash: string;
  nonce: number;
}

export interface Blockchain {
  chain: Block[];
  difficulty: number;
}

// Mining types
export interface MiningStatus {
  is_mining: boolean;
  current_nonce: number;
  hash_rate: number;
  elapsed_ms: number;
  target_difficulty: number;
}

export interface MiningProgress {
  nonce: number;
  hashRate: number;
  elapsed: number;
}

// Network types (for future tabula-kb integration)
export interface PeerInfo {
  id: string;
  address: string;
  status: 'connected' | 'disconnected' | 'syncing';
  latency_ms: number;
  last_seen: number;
}

export interface NodeStatus {
  node_id: string;
  is_running: boolean;
  peers_count: number;
  chain_height: number;
  synced: boolean;
}

export interface ChainInfo {
  height: number;
  difficulty: number;
  total_transactions: number;
  last_block_time: number;
}

// Knowledge base types (for tabula-kb integration)
export interface Entity {
  id: string;
  entity_type: string;
  data: Record<string, unknown>;
  created_at: number;
  updated_at: number;
}

export interface QueryResult {
  entities: Entity[];
  total: number;
  query_time_ms: number;
}

// UI State types
export type TabId = 'blockchain' | 'mining' | 'network' | 'knowledge-base';

export interface AppSettings {
  theme: 'light' | 'dark' | 'system';
  autoRefresh: boolean;
  refreshInterval: number;
}

// Event types for Tauri events
export interface BlockMinedEvent {
  block: Block;
}

export interface MiningProgressEvent {
  nonce: number;
  hashRate: number;
  elapsed: number;
}

// Utility types
export type LoadingState = 'idle' | 'loading' | 'success' | 'error';

export interface AsyncState<T> {
  data: T | null;
  status: LoadingState;
  error: string | null;
}
