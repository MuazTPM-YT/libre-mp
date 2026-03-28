import { useState, useEffect, useCallback } from 'react';
import {
  HelpCircle, RefreshCw, Settings, Wifi, WifiOff, MonitorDot, Search,
  FolderOpen, Signal, Radio, Zap, Clock, ArrowRight,
  Sun, Volume2, Gauge, Monitor, RotateCcw
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import './index.css';

import { SettingsModal } from './components/SettingsModal';
import { ConnectionModeModal } from './components/ConnectionModeModal';
import { HelpModal } from './components/HelpModal';

interface WifiNetwork {
  ssid: string;
  bssid: string;
  signal: number;
  security: string;
  is_projector: boolean;
}

interface NetworkItem {
  id: string;
  name: string;
  ssid: string;
  signal: number;
  security: string;
  is_projector: boolean;
}


function App() {
  const [networks, setNetworks] = useState<NetworkItem[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  const [connectionMode, setConnectionMode] = useState<'quick' | 'advanced'>('quick');

  // Connection state
  const [connectedSSID, setConnectedSSID] = useState<string | null>(null);
  const [connectingSSID, setConnectingSSID] = useState<string | null>(null);
  const [recentConnections, setRecentConnections] = useState<NetworkItem[]>([]);
  const [autoReconnect] = useState(true);

  // UI state
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isConnectionModeOpen, setIsConnectionModeOpen] = useState(false);
  const [isHelpOpen, setIsHelpOpen] = useState(false);
  const [activeSection, setActiveSection] = useState<'discovery' | 'profiles'>('discovery');

  const scanNetworks = useCallback(async () => {
    try {
      setIsScanning(true);
      const results: WifiNetwork[] = await invoke('scan_wifi_networks');
      const items: NetworkItem[] = results.map((n) => ({
        id: n.bssid,
        name: n.ssid || "Hidden Network",
        ssid: n.ssid,
        signal: n.signal,
        security: n.security,
        is_projector: n.is_projector,
      }));

      // Also discover projectors via UDP broadcast
      try {
        const projectors: any[] = await invoke('discover_projectors');
        for (const p of projectors) {
          // Only add if not already in the Wi-Fi list
          const exists = items.some(n => n.name === p.name || n.ssid === p.name);
          if (!exists) {
            items.push({
              id: `proj-${p.ip}`,
              name: p.name,
              ssid: p.name,
              signal: 100,
              security: 'Projector',
              is_projector: true,
            });
          } else {
            // Mark existing Wi-Fi entry as a projector if it matches
            const idx = items.findIndex(n => n.name === p.name || n.ssid === p.name);
            if (idx >= 0) items[idx].is_projector = true;
          }
        }
      } catch {
        // UDP discovery may fail silently
      }

      setNetworks(items);

      // Auto-reconnect logic
      if (autoReconnect && !connectedSSID && recentConnections.length > 0) {
        const lastSSID = recentConnections[0].ssid;
        const match = items.find(n => n.ssid === lastSSID);
        if (match) {
          handleConnect(match);
        }
      }
    } catch {
      // silent fail
    } finally {
      setIsScanning(false);
    }
  }, [autoReconnect, connectedSSID, recentConnections]);

  useEffect(() => {
    scanNetworks();
    const interval = setInterval(scanNetworks, 12000);
    return () => clearInterval(interval);
  }, [scanNetworks]);

  const handleConnect = async (network: NetworkItem) => {
    setConnectingSSID(network.ssid);
    try {
      const success: boolean = await invoke('connect_to_wifi', { ssid: network.ssid });
      if (success) {
        setConnectedSSID(network.ssid);
        // Add to recent
        setRecentConnections(prev => {
          const filtered = prev.filter(r => r.ssid !== network.ssid);
          return [network, ...filtered].slice(0, 5);
        });

        // Try to get friendly name via UDP discovery now that we are connected
        try {
          const discovered: any[] = await invoke('discover_projectors');
          if (discovered.length > 0) {
            // If we found a projector on the new network, use its name
            setNetworks(prev => prev.map(n => 
              n.ssid === network.ssid ? { ...n, name: discovered[0].name } : n
            ));
          }
        } catch (e) {
          console.error("UDP Discovery failed", e);
        }
      }
    } catch {
      // silent
    } finally {
      setConnectingSSID(null);
    }
  };

  const handleDisconnect = () => {
    setConnectedSSID(null);
  };

  // Filtering
  const filtered = networks
    .filter(n => n.name.toLowerCase().includes(searchQuery.toLowerCase()))
    .sort((a, b) => {
      // Projectors always first
      if (a.is_projector && !b.is_projector) return -1;
      if (!a.is_projector && b.is_projector) return 1;
      return b.signal - a.signal;
    });

  const signalLevel = (s: number) => s > 80 ? 5 : s > 60 ? 4 : s > 40 ? 3 : s > 20 ? 2 : 1;
  const signalColor = (s: number) => s > 60 ? 'high' : s > 30 ? 'mid' : 'low';

  const connectedNetwork = networks.find(n => n.ssid === connectedSSID);

  return (
    <div className="app">
      {/* ====== CONNECTION STATUS BANNER ====== */}
      <header className={`status-banner ${connectedSSID ? 'connected' : connectingSSID ? 'connecting' : 'idle'}`}>
        <div className="banner-left">
          <div className={`banner-dot ${connectedSSID ? 'on' : connectingSSID ? 'pulse' : ''}`} />
          {connectedSSID ? (
            <>
              <Wifi size={14} />
              <span>Connected to <strong>{connectedNetwork?.name || connectedSSID}</strong></span>
            </>
          ) : connectingSSID ? (
            <>
              <RotateCcw size={14} className="spinning" />
              <span>Connecting to <strong>{connectingSSID}</strong>...</span>
            </>
          ) : (
            <>
              <WifiOff size={14} />
              <span>Not connected</span>
            </>
          )}
        </div>
        <div className="banner-right">
          {connectedSSID && (
            <button className="banner-btn" onClick={handleDisconnect}>Disconnect</button>
          )}
          {autoReconnect && (
            <span className="banner-tag">
              <Zap size={10} />
              Auto
            </span>
          )}
        </div>
      </header>

      <div className="app-body">
        {/* ====== SIDEBAR ====== */}
        <aside className="sidebar">
          <div className="brand">
            <div className="brand-icon"><Radio size={16} /></div>
            <div className="brand-text">
              <span className="brand-name">LibreMP</span>
              <span className="brand-ver">v1.0</span>
            </div>
          </div>

          <nav className="nav">
            <button className={`nav-btn ${activeSection === 'discovery' ? 'active' : ''}`} onClick={() => setActiveSection('discovery')}>
              <Search size={16} />
              <span>Discovery</span>
              {networks.length > 0 && <span className="badge">{networks.length}</span>}
            </button>
            <button className={`nav-btn ${activeSection === 'profiles' ? 'active' : ''}`} onClick={() => setActiveSection('profiles')}>
              <FolderOpen size={16} />
              <span>Profiles</span>
            </button>
          </nav>

          {/* Quick Settings */}
          <div className="sidebar-section">
            <h4 className="section-label">Quick Settings</h4>
            <div className="quick-setting">
              <Monitor size={14} />
              <span>Display</span>
              <span className="qs-value">Operations</span>
            </div>
            <div className="quick-setting">
              <Sun size={14} />
              <span>Brightness</span>
              <span className="qs-value">80%</span>
            </div>
            <div className="quick-setting">
              <Volume2 size={14} />
              <span>Audio</span>
              <span className="qs-value">On</span>
            </div>
            <div className="quick-setting">
              <Gauge size={14} />
              <span>Bandwidth</span>
              <span className="qs-value">15Mbps</span>
            </div>
          </div>

          <div className="sidebar-spacer" />

          <div className="sidebar-footer">
            <button className="nav-btn subtle" onClick={() => setIsSettingsOpen(true)}>
              <Settings size={16} />
              <span>Settings</span>
            </button>
          </div>
        </aside>

        {/* ====== MAIN CONTENT ====== */}
        <main className="main">
          {/* Header Row */}
          <div className="main-header">
            <div>
              <h1 className="page-title">
                <Signal size={22} />
                Network Discovery
              </h1>
              <p className="page-sub">Find and connect to available projectors</p>
            </div>
            <div className="header-tools">
              <div className="search-pill">
                <Search size={15} />
                <input
                  type="text"
                  placeholder="Search networks..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                />
              </div>
              <button className="icon-circle" onClick={scanNetworks} disabled={isScanning} title="Refresh">
                <RefreshCw size={15} className={isScanning ? 'spinning' : ''} />
              </button>
            </div>
          </div>



          {/* Scrollable Body */}
          <div className="main-scroll">
            {/* Recent Connections */}
            {recentConnections.length > 0 && (
              <section className="recent-section">
                <h3 className="section-heading">
                  <Clock size={14} />
                  Recent Connections
                </h3>
                <div className="recent-row">
                  {recentConnections.map(r => (
                    <button key={r.ssid} className="recent-chip" onClick={() => handleConnect(r)}>
                      <MonitorDot size={14} />
                      <span>{r.name}</span>
                      <Zap size={12} />
                    </button>
                  ))}
                </div>
              </section>
            )}

            {/* Available Networks */}
            <section className="networks-section">
              <h3 className="section-heading">
                <Wifi size={14} />
                Available Networks
                <span className="heading-count">{filtered.length}</span>
              </h3>

              {filtered.length === 0 ? (
                <div className="empty-state">
                  <div className="empty-icon"><Signal size={28} /></div>
                  <h4>{isScanning ? 'Scanning for networks...' : 'No networks found'}</h4>
                  <p>Ensure your Wi-Fi adapter is active and click refresh.</p>
                </div>
              ) : (
                <div className="network-list">
                  {filtered.map(n => {
                    const isConnected = n.ssid === connectedSSID;
                    const isConnecting = n.ssid === connectingSSID;
                    return (
                      <div key={n.id} className={`net-card ${isConnected ? 'connected' : ''} ${n.is_projector ? 'projector' : ''}`}>
                        <div className="net-icon">
                          {n.is_projector ? <MonitorDot size={20} /> : <Wifi size={20} />}
                        </div>
                        <div className="net-info">
                          <div className="net-name">
                            {n.name}
                            {n.is_projector && <span className="projector-tag">Projector</span>}
                            {isConnected && <span className="connected-tag">Connected</span>}
                          </div>
                          <div className="net-meta">
                            <span>{n.security}</span>
                            <span className="meta-dot">·</span>
                            <span>{n.signal}% signal</span>
                          </div>
                        </div>
                        <div className="net-signal">
                          <div className={`signal-bars ${signalColor(n.signal)}`}>
                            {[1,2,3,4,5].map(i => (
                              <div key={i} className={`bar ${i <= signalLevel(n.signal) ? 'on' : ''}`} />
                            ))}
                          </div>
                        </div>
                        <button
                          className={`connect-btn-sm ${isConnected ? 'active' : ''}`}
                          onClick={(e) => { e.stopPropagation(); isConnected ? handleDisconnect() : handleConnect(n); }}
                          disabled={isConnecting}
                        >
                          {isConnecting ? (
                            <RotateCcw size={14} className="spinning" />
                          ) : isConnected ? (
                            <>
                              <Wifi size={14} />
                              <span>Connected</span>
                            </>
                          ) : (
                            <>
                              <ArrowRight size={14} />
                              <span>Connect</span>
                            </>
                          )}
                        </button>
                      </div>
                    );
                  })}
                </div>
              )}
            </section>
          </div>
        </main>
      </div>

      {/* ====== MODALS ====== */}
      <SettingsModal isOpen={isSettingsOpen} onClose={() => setIsSettingsOpen(false)} />
      <ConnectionModeModal isOpen={isConnectionModeOpen} onClose={() => setIsConnectionModeOpen(false)} mode={connectionMode} setMode={setConnectionMode} />
      <HelpModal isOpen={isHelpOpen} onClose={() => setIsHelpOpen(false)} />

      {/* ====== HELP FAB ====== */}
      {!isHelpOpen && (
        <button className="help-fab" onClick={() => setIsHelpOpen(true)} title="Help &amp; Guide">
          <HelpCircle size={20} />
        </button>
      )}
    </div>
  );
}

export default App;
