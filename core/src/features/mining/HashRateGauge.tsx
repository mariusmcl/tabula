import { memo, useMemo } from 'react';
import { Activity } from 'lucide-react';
import './HashRateGauge.css';

interface HashRateGaugeProps {
  hashRate: number;
  maxRate?: number;
  isActive: boolean;
}

export const HashRateGauge = memo(function HashRateGauge({
  hashRate,
  maxRate = 10000,
  isActive,
}: HashRateGaugeProps) {
  const percentage = useMemo(() => {
    return Math.min((hashRate / maxRate) * 100, 100);
  }, [hashRate, maxRate]);

  const level = useMemo(() => {
    if (percentage >= 70) return 'high';
    if (percentage >= 40) return 'medium';
    return 'low';
  }, [percentage]);

  const formatHashRate = (rate: number): string => {
    if (rate >= 1000000) {
      return `${(rate / 1000000).toFixed(2)} MH/s`;
    }
    if (rate >= 1000) {
      return `${(rate / 1000).toFixed(2)} KH/s`;
    }
    return `${rate.toFixed(0)} H/s`;
  };

  // Calculate gauge arc
  const arcLength = 251.2; // Circumference of circle with r=40
  const dashOffset = arcLength - (arcLength * percentage) / 100;

  return (
    <div className={`hashrate-gauge ${isActive ? 'active' : ''}`}>
      <div className="gauge-visual">
        <svg viewBox="0 0 100 100" className="gauge-svg">
          {/* Background arc */}
          <circle
            cx="50"
            cy="50"
            r="40"
            fill="none"
            stroke="var(--surface-control)"
            strokeWidth="8"
            strokeDasharray={arcLength}
            strokeDashoffset="0"
            transform="rotate(-90 50 50)"
            strokeLinecap="round"
          />
          {/* Progress arc */}
          <circle
            cx="50"
            cy="50"
            r="40"
            fill="none"
            stroke={`var(--hashrate-${level})`}
            strokeWidth="8"
            strokeDasharray={arcLength}
            strokeDashoffset={dashOffset}
            transform="rotate(-90 50 50)"
            strokeLinecap="round"
            className="gauge-progress"
          />
        </svg>
        <div className="gauge-center">
          <Activity className={isActive ? 'pulsing' : ''} />
        </div>
      </div>

      <div className="gauge-info">
        <span className="gauge-value">{formatHashRate(hashRate)}</span>
        <span className="gauge-label">Hash Rate</span>
      </div>

      <div className={`gauge-indicator ${level}`}>
        <span className="indicator-dot" />
        <span className="indicator-label">
          {level === 'high' ? 'Excellent' : level === 'medium' ? 'Good' : 'Low'}
        </span>
      </div>
    </div>
  );
});
