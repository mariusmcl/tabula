import { useState, memo } from 'react';
import { Play, Square, AlertCircle } from 'lucide-react';
import { HashRateGauge } from './HashRateGauge';
import { MiningProgress } from './MiningProgress';
import { useMining } from './useMining';
import './MiningDashboard.css';

interface MiningDashboardProps {
  difficulty: number;
  onBlockMined?: () => void;
}

export const MiningDashboard = memo(function MiningDashboard({
  difficulty,
}: MiningDashboardProps) {
  const { isMining, progress, hashRate, elapsedTime, start, stop, error } = useMining();
  const [blockData, setBlockData] = useState('');

  const handleStart = async () => {
    if (blockData.trim()) {
      await start(blockData.trim());
      setBlockData('');
    }
  };

  const handleStop = async () => {
    await stop();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey && !isMining) {
      e.preventDefault();
      handleStart();
    }
  };

  return (
    <div className="mining-dashboard">
      <div className="dashboard-header">
        <h2>Mining Dashboard</h2>
        <div className={`mining-status ${isMining ? 'active' : 'idle'}`}>
          <span className="status-dot" />
          <span className="status-text">{isMining ? 'Mining' : 'Idle'}</span>
        </div>
      </div>

      {error && (
        <div className="mining-error">
          <AlertCircle size={16} />
          <span>{error}</span>
        </div>
      )}

      <div className="dashboard-grid">
        <div className="dashboard-controls">
          <div className="control-section">
            <label className="control-label">Block Data</label>
            <div className="input-wrapper">
              <textarea
                placeholder="Enter data to include in the new block..."
                value={blockData}
                onChange={(e) => setBlockData(e.target.value)}
                onKeyDown={handleKeyDown}
                disabled={isMining}
                rows={3}
                maxLength={500}
              />
            </div>
            <span className="char-count">{blockData.length}/500</span>
          </div>

          <div className="control-actions">
            {isMining ? (
              <button className="danger large" onClick={handleStop}>
                <Square size={18} />
                Stop Mining
              </button>
            ) : (
              <button
                className="mining large"
                onClick={handleStart}
                disabled={!blockData.trim()}
              >
                <Play size={18} />
                Start Mining
              </button>
            )}
          </div>

          <div className="mining-info">
            <div className="info-row">
              <span className="info-label">Difficulty</span>
              <span className="info-value">{difficulty}</span>
            </div>
            <div className="info-row">
              <span className="info-label">Target</span>
              <code className="info-value code">{'0'.repeat(difficulty)}...</code>
            </div>
          </div>
        </div>

        <div className="dashboard-stats">
          <HashRateGauge hashRate={hashRate} isActive={isMining} />

          <MiningProgress
            nonce={progress?.nonce ?? 0}
            elapsed={elapsedTime}
            difficulty={difficulty}
            isActive={isMining}
          />
        </div>
      </div>
    </div>
  );
});
