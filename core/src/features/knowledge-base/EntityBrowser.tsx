import { useState, memo } from 'react';
import { Search, Database, RefreshCw, FileText, Box } from 'lucide-react';
import type { Entity } from '../../types';
import './EntityBrowser.css';

interface EntityBrowserProps {
  entities: Entity[];
  isLoading: boolean;
  isConnected: boolean;
  onSearch: (query: string) => void;
  onSelectType: (type: string) => void;
  onRefresh: () => void;
}

const ENTITY_TYPES = [
  { id: 'document', label: 'Documents', icon: FileText },
  { id: 'concept', label: 'Concepts', icon: Box },
  { id: 'relation', label: 'Relations', icon: Database },
];

export const EntityBrowser = memo(function EntityBrowser({
  entities,
  isLoading,
  isConnected,
  onSearch,
  onSelectType,
  onRefresh,
}: EntityBrowserProps) {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedType, setSelectedType] = useState<string | null>(null);

  const handleSearch = () => {
    if (searchQuery.trim()) {
      onSearch(searchQuery.trim());
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleSearch();
    }
  };

  const handleTypeSelect = (type: string) => {
    setSelectedType(type);
    onSelectType(type);
  };

  return (
    <div className="entity-browser">
      <div className="browser-header">
        <h2>Knowledge Base Explorer</h2>
        <div className="header-actions">
          <div className={`connection-badge ${isConnected ? 'connected' : ''}`}>
            <span className="badge-dot" />
            {isConnected ? 'Connected' : 'Disconnected'}
          </div>
          <button
            className="icon-button"
            onClick={onRefresh}
            disabled={isLoading}
            title="Refresh"
          >
            <RefreshCw className={isLoading ? 'spinning' : ''} />
          </button>
        </div>
      </div>

      <div className="browser-controls">
        <div className="search-bar">
          <div className="input-wrapper with-icon">
            <span className="input-icon">
              <Search size={18} />
            </span>
            <input
              type="text"
              placeholder="Search knowledge base..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              onKeyDown={handleKeyDown}
              disabled={!isConnected}
            />
          </div>
          <button
            className="primary"
            onClick={handleSearch}
            disabled={!isConnected || !searchQuery.trim()}
          >
            Search
          </button>
        </div>

        <div className="type-filters">
          {ENTITY_TYPES.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              className={`type-filter ${selectedType === id ? 'active' : ''}`}
              onClick={() => handleTypeSelect(id)}
              disabled={!isConnected}
            >
              <Icon size={16} />
              {label}
            </button>
          ))}
        </div>
      </div>

      <div className="entity-list">
        {isLoading ? (
          <div className="loading-state">
            <div className="spinner large" />
            <span>Loading entities...</span>
          </div>
        ) : !isConnected ? (
          <div className="empty-state">
            <Database size={48} />
            <h3>Not Connected</h3>
            <p>Connect to a tabula-kb node to browse the knowledge base.</p>
          </div>
        ) : entities.length === 0 ? (
          <div className="empty-state">
            <Search size={48} />
            <h3>No entities found</h3>
            <p>Try searching or selecting an entity type to browse.</p>
          </div>
        ) : (
          <ul className="entities">
            {entities.map((entity) => (
              <li key={entity.id} className="entity-item">
                <div className="entity-header">
                  <span className="entity-type badge">{entity.entity_type}</span>
                  <span className="entity-id">{entity.id}</span>
                </div>
                <div className="entity-data">
                  <pre>{JSON.stringify(entity.data, null, 2)}</pre>
                </div>
                <div className="entity-meta">
                  <span>Created: {new Date(entity.created_at).toLocaleString()}</span>
                  <span>Updated: {new Date(entity.updated_at).toLocaleString()}</span>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
});
