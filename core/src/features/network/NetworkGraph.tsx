import { memo, useMemo } from 'react';
import type { PeerInfo } from '../../types';
import './NetworkGraph.css';

interface NetworkGraphProps {
  peers: PeerInfo[];
  nodeId: string;
  isConnected: boolean;
}

interface NodePosition {
  x: number;
  y: number;
}

export const NetworkGraph = memo(function NetworkGraph({
  peers,
  nodeId: _nodeId,
  isConnected,
}: NetworkGraphProps) {
  const centerX = 200;
  const centerY = 150;
  const radius = 100;

  const peerPositions = useMemo((): Map<string, NodePosition> => {
    const positions = new Map<string, NodePosition>();

    peers.forEach((peer, index) => {
      const angle = (index / peers.length) * 2 * Math.PI - Math.PI / 2;
      positions.set(peer.id, {
        x: centerX + radius * Math.cos(angle),
        y: centerY + radius * Math.sin(angle),
      });
    });

    return positions;
  }, [peers]);

  const getStatusColor = (status: PeerInfo['status']): string => {
    switch (status) {
      case 'connected':
        return 'var(--node-connected)';
      case 'syncing':
        return 'var(--node-syncing)';
      case 'disconnected':
        return 'var(--node-disconnected)';
      default:
        return 'var(--status-unknown)';
    }
  };

  return (
    <div className="network-graph">
      <svg viewBox="0 0 400 300" className="graph-svg">
        {/* Connection lines */}
        {peers.map((peer) => {
          const pos = peerPositions.get(peer.id);
          if (!pos) return null;

          return (
            <line
              key={`line-${peer.id}`}
              x1={centerX}
              y1={centerY}
              x2={pos.x}
              y2={pos.y}
              className={`connection-line ${peer.status}`}
              stroke={getStatusColor(peer.status)}
              strokeWidth="2"
              strokeDasharray={peer.status === 'syncing' ? '5,5' : 'none'}
            />
          );
        })}

        {/* Center node (local) */}
        <g className="node local-node">
          <circle
            cx={centerX}
            cy={centerY}
            r="24"
            className={`node-circle ${isConnected ? 'connected' : 'disconnected'}`}
          />
          <text x={centerX} y={centerY + 5} className="node-label">
            You
          </text>
        </g>

        {/* Peer nodes */}
        {peers.map((peer) => {
          const pos = peerPositions.get(peer.id);
          if (!pos) return null;

          return (
            <g key={peer.id} className={`node peer-node ${peer.status}`}>
              <circle cx={pos.x} cy={pos.y} r="18" className="node-circle" />
              <text x={pos.x} y={pos.y + 4} className="node-label">
                {peer.id.slice(0, 4)}
              </text>

              {/* Latency indicator */}
              <text x={pos.x} y={pos.y + 32} className="node-latency">
                {peer.latency_ms}ms
              </text>
            </g>
          );
        })}

        {/* Legend */}
        <g className="graph-legend" transform="translate(10, 260)">
          <circle cx="8" cy="8" r="6" fill="var(--node-connected)" />
          <text x="20" y="12" className="legend-text">
            Connected
          </text>

          <circle cx="100" cy="8" r="6" fill="var(--node-syncing)" />
          <text x="112" y="12" className="legend-text">
            Syncing
          </text>

          <circle cx="180" cy="8" r="6" fill="var(--node-disconnected)" />
          <text x="192" y="12" className="legend-text">
            Disconnected
          </text>
        </g>
      </svg>

      {peers.length === 0 && (
        <div className="graph-empty">
          <span>No peers connected</span>
        </div>
      )}
    </div>
  );
});
