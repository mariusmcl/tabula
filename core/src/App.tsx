import { useState, useCallback } from 'react';
import {
  Boxes,
  Pickaxe,
  Globe,
} from 'lucide-react';
import { AppLayout, useWindowSize } from './features/layout';
import { BlockchainStats, BlockExplorer, useBlockchain } from './features/blockchain';
import { MiningDashboard } from './features/mining';
import { NetworkDashboard } from './features/network';
import type { TabId } from './types';
import './App.css';

function App() {
  const [activeTab, setActiveTab] = useState<TabId>('blockchain');
  const { isPhone } = useWindowSize();
  const blockchain = useBlockchain();

  const handleTabChange = useCallback((tab: TabId) => {
    setActiveTab(tab);
  }, []);

  const renderTopbar = () => (
    <>
      <div className="topbar-left">
        <div className="logo">
          <svg viewBox="0 0 100 100" width="28" height="28">
            <defs>
              <linearGradient id="chainGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                <stop offset="0%" style={{ stopColor: '#62b7ff' }} />
                <stop offset="100%" style={{ stopColor: '#4fe3a3' }} />
              </linearGradient>
            </defs>
            <rect x="10" y="10" width="30" height="30" rx="4" fill="url(#chainGrad)" />
            <rect x="35" y="35" width="30" height="30" rx="4" fill="url(#chainGrad)" opacity="0.8" />
            <rect x="60" y="60" width="30" height="30" rx="4" fill="url(#chainGrad)" opacity="0.6" />
          </svg>
          <span>Tabula</span>
        </div>
      </div>

      {!isPhone && (
        <nav className="nav-tabs">
          <button
            className={`nav-tab ${activeTab === 'blockchain' ? 'active' : ''}`}
            onClick={() => handleTabChange('blockchain')}
          >
            <Boxes size={16} />
            Blockchain
          </button>
          <button
            className={`nav-tab ${activeTab === 'mining' ? 'active' : ''}`}
            onClick={() => handleTabChange('mining')}
          >
            <Pickaxe size={16} />
            Mining
          </button>
          <button
            className={`nav-tab ${activeTab === 'network' ? 'active' : ''}`}
            onClick={() => handleTabChange('network')}
          >
            <Globe size={16} />
            Network
          </button>
        </nav>
      )}

      <div className="topbar-right">
        <div className={`chain-status ${blockchain.isValid ? 'valid' : 'invalid'}`}>
          <span className="status-dot" />
          <span className="status-text">
            {blockchain.isValid ? 'Chain Valid' : 'Chain Invalid'}
          </span>
        </div>
      </div>
    </>
  );

  const renderContent = () => {
    switch (activeTab) {
      case 'blockchain':
        return (
          <div className="page blockchain-page">
            <BlockchainStats
              blocks={blockchain.blocks}
              difficulty={blockchain.difficulty}
              isValid={blockchain.isValid}
              isLoading={blockchain.status === 'loading'}
            />
            <BlockExplorer
              blocks={blockchain.blocks}
              isLoading={blockchain.status === 'loading'}
              isMining={blockchain.isMining}
              onRefresh={blockchain.refresh}
              onMineBlock={blockchain.mineBlock}
            />
          </div>
        );
      case 'mining':
        return (
          <div className="page mining-page">
            <MiningDashboard
              difficulty={blockchain.difficulty}
              onBlockMined={blockchain.refresh}
            />
          </div>
        );
      case 'network':
        return (
          <div className="page network-page">
            <NetworkDashboard />
          </div>
        );
      default:
        return null;
    }
  };

  const renderBottomNav = () => (
    <>
      <button
        className={`bottom-nav-item ${activeTab === 'blockchain' ? 'active' : ''}`}
        onClick={() => handleTabChange('blockchain')}
      >
        <Boxes />
        <span>Blocks</span>
      </button>
      <button
        className={`bottom-nav-item ${activeTab === 'mining' ? 'active' : ''}`}
        onClick={() => handleTabChange('mining')}
      >
        <Pickaxe />
        <span>Mining</span>
      </button>
      <button
        className={`bottom-nav-item ${activeTab === 'network' ? 'active' : ''}`}
        onClick={() => handleTabChange('network')}
      >
        <Globe />
        <span>Network</span>
      </button>
    </>
  );

  return (
    <div className="app" data-theme="dark">
      <AppLayout
        topbar={renderTopbar()}
        content={renderContent()}
        bottomNav={isPhone ? renderBottomNav() : undefined}
      />
    </div>
  );
}

export default App;
