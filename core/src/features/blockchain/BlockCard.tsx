import { memo } from 'react';
import { Box, Hash, Clock, Link2, Pickaxe } from 'lucide-react';
import type { Block } from '../../types';
import { formatTimestamp, truncateHash, formatNonce } from '../../services/blockchain';
import './BlockCard.css';

interface BlockCardProps {
  block: Block;
  isLatest?: boolean;
}

export const BlockCard = memo(function BlockCard({ block, isLatest }: BlockCardProps) {
  const isGenesis = block.index === 0;

  return (
    <article className={`block-card ${isGenesis ? 'genesis' : ''} ${isLatest ? 'latest' : ''}`}>
      <header className="block-card-header">
        <div className="block-index">
          <Box size={16} />
          <span>Block #{block.index}</span>
        </div>
        <div className="block-badges">
          {isGenesis && <span className="badge genesis">Genesis</span>}
          {isLatest && <span className="badge success">Latest</span>}
        </div>
      </header>

      <div className="block-card-content">
        <div className="block-data">
          <p className="data-content">{block.data}</p>
        </div>

        <div className="block-details">
          <div className="detail-row">
            <Clock size={14} />
            <span className="detail-label">Timestamp</span>
            <span className="detail-value">{formatTimestamp(block.timestamp)}</span>
          </div>

          <div className="detail-row">
            <Hash size={14} />
            <span className="detail-label">Hash</span>
            <span className="detail-value hash" title={block.hash}>
              {truncateHash(block.hash, 12)}
            </span>
          </div>

          <div className="detail-row">
            <Link2 size={14} />
            <span className="detail-label">Previous</span>
            <span className="detail-value hash" title={block.previous_hash}>
              {truncateHash(block.previous_hash, 12)}
            </span>
          </div>

          <div className="detail-row">
            <Pickaxe size={14} />
            <span className="detail-label">Nonce</span>
            <span className="detail-value nonce">{formatNonce(block.nonce)}</span>
          </div>
        </div>
      </div>

      {!isGenesis && (
        <footer className="block-card-footer">
          <div className="hash-preview">
            <span className="hash-label">Full Hash:</span>
            <code className="hash-value">{block.hash}</code>
          </div>
        </footer>
      )}
    </article>
  );
});
