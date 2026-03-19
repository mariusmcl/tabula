import { memo } from 'react';
import { Boxes, Gauge, Shield, Clock } from 'lucide-react';
import type { Block } from '../../types';
import { formatTimestamp } from '../../services/blockchain';
import './BlockchainStats.css';

interface BlockchainStatsProps {
  blocks: Block[];
  difficulty: number;
  isValid: boolean;
  isLoading?: boolean;
}

export const BlockchainStats = memo(function BlockchainStats({
  blocks,
  difficulty,
  isValid,
  isLoading,
}: BlockchainStatsProps) {
  const latestBlock = blocks[blocks.length - 1];
  const totalNonces = blocks.reduce((sum, block) => sum + block.nonce, 0);

  return (
    <div className="stats-grid">
      <div className="stat-card">
        <div className="stat-icon">
          <Boxes />
        </div>
        <span className="stat-label">Total Blocks</span>
        <span className="stat-value">
          {isLoading ? '...' : blocks.length}
        </span>
      </div>

      <div className="stat-card">
        <div className="stat-icon">
          <Gauge />
        </div>
        <span className="stat-label">Difficulty</span>
        <span className="stat-value">
          {isLoading ? '...' : difficulty}
        </span>
        <span className="stat-description">
          Leading zeros required
        </span>
      </div>

      <div className="stat-card">
        <div className={`stat-icon ${isValid ? 'valid' : 'invalid'}`}>
          <Shield />
        </div>
        <span className="stat-label">Chain Status</span>
        <span className={`stat-value status ${isValid ? 'valid' : 'invalid'}`}>
          {isLoading ? '...' : isValid ? 'Valid' : 'Invalid'}
        </span>
      </div>

      <div className="stat-card">
        <div className="stat-icon">
          <Clock />
        </div>
        <span className="stat-label">Last Block</span>
        <span className="stat-value small">
          {isLoading ? '...' : latestBlock ? formatTimestamp(latestBlock.timestamp) : 'N/A'}
        </span>
      </div>

      <div className="stat-card wide">
        <div className="stat-icon">
          <Gauge />
        </div>
        <span className="stat-label">Total Mining Work</span>
        <span className="stat-value">
          {isLoading ? '...' : totalNonces.toLocaleString()}
        </span>
        <span className="stat-description">
          Total nonces computed
        </span>
      </div>
    </div>
  );
});
