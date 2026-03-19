import { useState, useMemo, memo } from 'react';
import { Search, RefreshCw, Plus, ChevronDown, ChevronUp } from 'lucide-react';
import type { Block } from '../../types';
import { BlockCard } from './BlockCard';
import './BlockExplorer.css';

interface BlockExplorerProps {
  blocks: Block[];
  isLoading: boolean;
  isMining: boolean;
  onRefresh: () => void;
  onMineBlock: (data: string) => void;
}

type SortOrder = 'newest' | 'oldest';

export const BlockExplorer = memo(function BlockExplorer({
  blocks,
  isLoading,
  isMining,
  onRefresh,
  onMineBlock,
}: BlockExplorerProps) {
  const [searchQuery, setSearchQuery] = useState('');
  const [sortOrder, setSortOrder] = useState<SortOrder>('newest');
  const [newBlockData, setNewBlockData] = useState('');
  const [showMineForm, setShowMineForm] = useState(false);

  const filteredBlocks = useMemo(() => {
    let result = [...blocks];

    // Filter by search query
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter(
        (block) =>
          block.data.toLowerCase().includes(query) ||
          block.hash.toLowerCase().includes(query) ||
          block.index.toString().includes(query)
      );
    }

    // Sort blocks
    if (sortOrder === 'newest') {
      result.sort((a, b) => b.index - a.index);
    } else {
      result.sort((a, b) => a.index - b.index);
    }

    return result;
  }, [blocks, searchQuery, sortOrder]);

  const handleMineBlock = () => {
    if (newBlockData.trim() && !isMining) {
      onMineBlock(newBlockData.trim());
      setNewBlockData('');
      setShowMineForm(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleMineBlock();
    }
  };

  const latestBlockIndex = blocks.length > 0 ? Math.max(...blocks.map((b) => b.index)) : -1;

  return (
    <div className="block-explorer">
      <div className="explorer-header">
        <h2>Block Explorer</h2>
        <div className="explorer-actions">
          <button
            className="icon-button"
            onClick={onRefresh}
            disabled={isLoading}
            title="Refresh blockchain"
          >
            <RefreshCw className={isLoading ? 'spinning' : ''} />
          </button>
          <button
            className={`secondary ${showMineForm ? 'active' : ''}`}
            onClick={() => setShowMineForm(!showMineForm)}
          >
            <Plus size={16} />
            Mine Block
          </button>
        </div>
      </div>

      {showMineForm && (
        <div className="mine-form">
          <div className="input-wrapper">
            <input
              type="text"
              placeholder="Enter block data..."
              value={newBlockData}
              onChange={(e) => setNewBlockData(e.target.value)}
              onKeyDown={handleKeyDown}
              disabled={isMining}
              maxLength={100}
            />
          </div>
          <button
            className="mining"
            onClick={handleMineBlock}
            disabled={isMining || !newBlockData.trim()}
          >
            {isMining ? 'Mining...' : 'Start Mining'}
          </button>
        </div>
      )}

      <div className="explorer-filters">
        <div className="input-wrapper with-icon">
          <span className="input-icon">
            <Search size={18} />
          </span>
          <input
            type="text"
            placeholder="Search by data, hash, or block number..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>

        <button
          className="ghost sort-button"
          onClick={() => setSortOrder(sortOrder === 'newest' ? 'oldest' : 'newest')}
        >
          {sortOrder === 'newest' ? (
            <>
              <ChevronDown size={16} />
              Newest First
            </>
          ) : (
            <>
              <ChevronUp size={16} />
              Oldest First
            </>
          )}
        </button>
      </div>

      <div className="blocks-grid">
        {filteredBlocks.length === 0 ? (
          <div className="empty-state">
            <Search size={48} />
            <h3>No blocks found</h3>
            <p>
              {searchQuery
                ? 'Try adjusting your search query'
                : 'Mine your first block to get started'}
            </p>
          </div>
        ) : (
          filteredBlocks.map((block) => (
            <BlockCard
              key={block.index}
              block={block}
              isLatest={block.index === latestBlockIndex}
            />
          ))
        )}
      </div>
    </div>
  );
});
