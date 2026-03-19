import { memo } from 'react';
import { Pickaxe, Clock, Hash } from 'lucide-react';
import './MiningProgress.css';

interface MiningProgressProps {
  nonce: number;
  elapsed: number;
  difficulty: number;
  isActive: boolean;
}

export const MiningProgress = memo(function MiningProgress({
  nonce,
  elapsed,
  difficulty,
  isActive,
}: MiningProgressProps) {
  const formatTime = (ms: number): string => {
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    const minutes = Math.floor(ms / 60000);
    const seconds = ((ms % 60000) / 1000).toFixed(0);
    return `${minutes}m ${seconds}s`;
  };

  const targetPattern = '0'.repeat(difficulty);

  return (
    <div className={`mining-progress ${isActive ? 'active' : ''}`}>
      <div className="progress-header">
        <Pickaxe className={isActive ? 'mining-icon' : ''} />
        <span className="progress-title">
          {isActive ? 'Mining in Progress' : 'Mining Idle'}
        </span>
      </div>

      <div className="progress-stats">
        <div className="progress-stat">
          <Hash size={16} />
          <div className="stat-content">
            <span className="stat-value">{nonce.toLocaleString()}</span>
            <span className="stat-label">Nonces Tried</span>
          </div>
        </div>

        <div className="progress-stat">
          <Clock size={16} />
          <div className="stat-content">
            <span className="stat-value">{formatTime(elapsed)}</span>
            <span className="stat-label">Elapsed Time</span>
          </div>
        </div>
      </div>

      <div className="progress-target">
        <span className="target-label">Target Pattern:</span>
        <code className="target-value">
          <span className="target-zeros">{targetPattern}</span>
          <span className="target-rest">{'x'.repeat(64 - difficulty)}</span>
        </code>
      </div>

      {isActive && (
        <div className="progress-animation">
          <div className="progress-bar">
            <div className="progress-fill" />
          </div>
          <span className="progress-text">Searching for valid hash...</span>
        </div>
      )}
    </div>
  );
});
