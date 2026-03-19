import { memo } from 'react';
import { Radio, Clock, Signal, SignalZero } from 'lucide-react';
import type { PeerInfo } from '../../types';
import './PeerList.css';

interface PeerListProps {
  peers: PeerInfo[];
  isLoading: boolean;
}

export const PeerList = memo(function PeerList({ peers, isLoading }: PeerListProps) {
  const formatLatency = (ms: number): string => {
    if (ms < 100) return 'Excellent';
    if (ms < 300) return 'Good';
    if (ms < 500) return 'Fair';
    return 'Poor';
  };

  const formatLastSeen = (timestamp: number): string => {
    const diff = Date.now() - timestamp;
    if (diff < 60000) return 'Just now';
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    return `${Math.floor(diff / 3600000)}h ago`;
  };

  const getStatusIcon = (status: PeerInfo['status']) => {
    switch (status) {
      case 'connected':
        return <Signal size={14} />;
      case 'syncing':
        return <Radio size={14} className="syncing" />;
      case 'disconnected':
        return <SignalZero size={14} />;
    }
  };

  return (
    <div className="peer-list">
      <div className="peer-list-header">
        <h3>Connected Peers</h3>
        <span className="peer-count">{peers.length}</span>
      </div>

      {isLoading ? (
        <div className="peer-list-loading">
          <div className="spinner" />
          <span>Loading peers...</span>
        </div>
      ) : peers.length === 0 ? (
        <div className="peer-list-empty">
          <SignalZero size={24} />
          <span>No peers connected</span>
        </div>
      ) : (
        <ul className="peers">
          {peers.map((peer) => (
            <li key={peer.id} className={`peer-item ${peer.status}`}>
              <div className="peer-info">
                <div className="peer-header">
                  <span className="peer-id">{peer.id}</span>
                  <div className={`peer-status ${peer.status}`}>
                    {getStatusIcon(peer.status)}
                    <span>{peer.status}</span>
                  </div>
                </div>
                <span className="peer-address">{peer.address}</span>
              </div>

              <div className="peer-stats">
                <div className="peer-stat">
                  <Signal size={12} />
                  <span className="stat-value">{peer.latency_ms}ms</span>
                  <span className="stat-label">{formatLatency(peer.latency_ms)}</span>
                </div>
                <div className="peer-stat">
                  <Clock size={12} />
                  <span className="stat-value">{formatLastSeen(peer.last_seen)}</span>
                </div>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
});
