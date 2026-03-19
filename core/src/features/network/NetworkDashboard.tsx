import { memo } from 'react';
import {
  Globe,
  RefreshCw,
  Wifi,
  WifiOff,
  Server,
  Users,
  Layers,
  Clock,
} from 'lucide-react';
import { NetworkGraph } from './NetworkGraph';
import { PeerList } from './PeerList';
import { useNetwork } from './useNetwork';
import './NetworkDashboard.css';

export const NetworkDashboard = memo(function NetworkDashboard() {
  const {
    isConnected,
    nodeStatus,
    peers,
    chainInfo,
    isLoading,
    error,
    refresh,
    connect,
  } = useNetwork();

  const formatLastBlockTime = (timestamp: number): string => {
    const diff = Date.now() - timestamp;
    if (diff < 60000) return 'Just now';
    if (diff < 3600000) return `${Math.floor(diff / 60000)} minutes ago`;
    return `${Math.floor(diff / 3600000)} hours ago`;
  };

  return (
    <div className="network-dashboard">
      <div className="dashboard-header">
        <h2>Network Status</h2>
        <div className="header-actions">
          <div className={`connection-status ${isConnected ? 'connected' : 'disconnected'}`}>
            {isConnected ? <Wifi size={16} /> : <WifiOff size={16} />}
            <span>{isConnected ? 'Connected' : 'Disconnected'}</span>
          </div>
          <button
            className="icon-button"
            onClick={refresh}
            disabled={isLoading}
            title="Refresh"
          >
            <RefreshCw className={isLoading ? 'spinning' : ''} />
          </button>
          {!isConnected && (
            <button className="primary" onClick={connect} disabled={isLoading}>
              Connect
            </button>
          )}
        </div>
      </div>

      {error && (
        <div className="network-error">
          <WifiOff size={16} />
          <span>{error}</span>
        </div>
      )}

      <div className="network-stats">
        <div className="stat-card">
          <div className="stat-icon">
            <Server />
          </div>
          <span className="stat-label">Node Status</span>
          <span className={`stat-value ${nodeStatus?.is_running ? 'running' : 'stopped'}`}>
            {isLoading ? '...' : nodeStatus?.is_running ? 'Running' : 'Stopped'}
          </span>
        </div>

        <div className="stat-card">
          <div className="stat-icon">
            <Users />
          </div>
          <span className="stat-label">Connected Peers</span>
          <span className="stat-value">
            {isLoading ? '...' : nodeStatus?.peers_count ?? 0}
          </span>
        </div>

        <div className="stat-card">
          <div className="stat-icon">
            <Layers />
          </div>
          <span className="stat-label">Chain Height</span>
          <span className="stat-value">
            {isLoading ? '...' : chainInfo?.height.toLocaleString() ?? 0}
          </span>
        </div>

        <div className="stat-card">
          <div className="stat-icon">
            <Clock />
          </div>
          <span className="stat-label">Last Block</span>
          <span className="stat-value small">
            {isLoading
              ? '...'
              : chainInfo
              ? formatLastBlockTime(chainInfo.last_block_time)
              : 'N/A'}
          </span>
        </div>
      </div>

      <div className="network-grid">
        <div className="network-visualization">
          <h3>Network Topology</h3>
          <NetworkGraph
            peers={peers}
            nodeId={nodeStatus?.node_id ?? 'local'}
            isConnected={isConnected}
          />
        </div>

        <div className="network-peers">
          <PeerList peers={peers} isLoading={isLoading} />
        </div>
      </div>

      {!isConnected && (
        <div className="network-info">
          <Globe size={20} />
          <div className="info-content">
            <h4>Connect to tabula-kb Network</h4>
            <p>
              Start a tabula-kb node to connect to the network and visualize
              peer connections. The node runs as a separate process.
            </p>
            <code>cd eksperimentering/tabula-kb && cargo run --bin node</code>
          </div>
        </div>
      )}
    </div>
  );
});
