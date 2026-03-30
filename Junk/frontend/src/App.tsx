import { useState, useEffect, useCallback } from 'react';
import {
  HelpCircle, RefreshCw, Settings, Wifi, WifiOff, MonitorDot, Search,
  Signal, Radio, Zap, ArrowRight,
  Sun, Moon, Volume2, Gauge, Monitor, RotateCcw, AlertTriangle, X, Shield, ShieldCheck
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import './index.css';

import { SettingsModal, type AppSettings, defaultSettings } from './components/SettingsModal';
import { ConnectionModeModal } from './components/ConnectionModeModal';
import { HelpModal } from './components/HelpModal';
import { PasswordModal } from './components/PasswordModal';

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
  ip?: string;
}


function App() {
  const [networks, setNetworks] = useState<NetworkItem[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  const [connectionMode, setConnectionMode] = useState<'quick' | 'advanced'>('quick');

  // Theme state
  const [theme, setTheme] = useState<'light' | 'dark'>(() => {
    const stored = localStorage.getItem('libre-mp-theme');
    if (stored === 'light' || stored === 'dark') return stored;
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  });

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('libre-mp-theme', theme);
  }, [theme]);

  const toggleTheme = () => setTheme(prev => prev === 'light' ? 'dark' : 'light');

  // App settings (shared between sidebar + modal)
  const [appSettings, setAppSettings] = useState<AppSettings>(defaultSettings);

  // Connection state
  const [connectedSSID, setConnectedSSID] = useState<string | null>(null);
  const [connectingSSID, setConnectingSSID] = useState<string | null>(null);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [isCasting, setIsCasting] = useState(false);

  // UI state
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isConnectionModeOpen, setIsConnectionModeOpen] = useState(false);
  const [isHelpOpen, setIsHelpOpen] = useState(false);
  const [passwordModalNet, setPasswordModalNet] = useState<NetworkItem | null>(null);
  const [isEditingBrightness, setIsEditingBrightness] = useState(false);
  const [brightnessInput, setBrightnessInput] = useState("");
  
  const isScanningRef = import.meta.env.DEV ? { current: false } : { current: false };

  const scanNetworks = useCallback(async () => {
    if (isScanningRef.current) return;
    isScanningRef.current = true;
    try {
      setIsScanning(true);
      let items: NetworkItem[] = [];

      // 1. Wi-Fi Scan
      try {
        const results: WifiNetwork[] = await invoke('scan_wifi_networks');
        items = results.map((n) => ({
          id: n.bssid || `wifi-${n.ssid}`,
          name: n.ssid || "Hidden Network",
          ssid: n.ssid,
          signal: n.signal,
          security: n.security,
          is_projector: n.is_projector,
        }));
      } catch (e) {
        console.warn("Wi-Fi scan failed:", e);
      }

      // 2. UDP Projector Discovery
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
              ip: p.ip,
            });
          } else {
            // Mark existing Wi-Fi entry as a projector if it matches
            const idx = items.findIndex(n => n.name === p.name || n.ssid === p.name);
            if (idx >= 0) {
              items[idx].is_projector = true;
              items[idx].ip = p.ip;
            }
          }
        }
      } catch (e) {
        console.warn("UDP discovery failed:", e);
      }

      // 3. Update State
      if (items.length > 0) {
        setNetworks(items);
      } else if (networks.length === 0) {
        setNetworks([]);
      }
    } catch {
      // silent fail
    } finally {
      setIsScanning(false);
      isScanningRef.current = false;
    }
  }, [networks.length]);

  const toggleCasting = async () => {
    if (isCasting) {
      try {
        await invoke('stop_casting');
        setIsCasting(false);
      } catch (e) {
        console.error("Failed to stop casting:", e);
      }
    } else {
      // Find a projector to cast to
      const projector = networks.find(n => n.ssid === connectedSSID && n.ip) || networks.find(n => n.is_projector && n.ip);
      if (projector && projector.ip) {
        try {
          await invoke('start_casting_async', { ip: projector.ip });
          setIsCasting(true);
        } catch (e) {
          setConnectionError("Failed to start casting. Make sure you are connected to the projector network.");
          console.error("Failed to start casting:", e);
        }
      } else {
        setConnectionError("No projector found to cast to. Please connect to a projector network first.");
      }
    }
  };

  useEffect(() => {
    scanNetworks();
    const interval = setInterval(scanNetworks, 12000);
    return () => clearInterval(interval);
  }, [scanNetworks]);

  // Auto-dismiss connection error after 8 seconds
  useEffect(() => {
    if (connectionError) {
      const timer = setTimeout(() => setConnectionError(null), 8000);
      return () => clearTimeout(timer);
    }
  }, [connectionError]);

  const handleNetworkClick = (network: NetworkItem) => {
    if (network.security !== 'Open' && network.security !== 'Projector') {
      setPasswordModalNet(network);
    } else {
      handleConnect(network, '');
    }
  };

  const handleConnect = async (network: NetworkItem, password?: string) => {
    setConnectingSSID(network.ssid);
    setConnectionError(null);
    try {
      const success: boolean = await invoke('connect_to_wifi', { ssid: network.ssid, password });
      if (success) {
        setConnectedSSID(network.ssid);

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
    } catch (err: any) {
      console.error("Connection failed:", err);
      setConnectionError(typeof err === 'string' ? err : (err?.message || 'Connection failed. Please try again.'));
    } finally {
      setConnectingSSID(null);
    }
  };

  const handleDisconnect = () => {
    setConnectedSSID(null);
    setConnectionError(null);
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
      <header className={`status-banner ${connectedSSID ? 'connected' : connectingSSID ? 'connecting' : connectionError ? 'error' : 'idle'}`}>
        <div className="banner-left">
          <div className={`banner-dot ${connectedSSID ? 'on' : connectingSSID ? 'pulse' : connectionError ? 'error' : ''}`} />
          {connectedSSID ? (
            <>
              <ShieldCheck size={14} />
              <span>Connected to <strong>{connectedNetwork?.name || connectedSSID}</strong></span>
            </>
          ) : connectingSSID ? (
            <>
              <RotateCcw size={14} className="spinning" />
              <span>Connecting to <strong>{connectingSSID}</strong>...</span>
            </>
          ) : connectionError ? (
            <>
              <AlertTriangle size={14} />
              <span className="banner-error-text">{connectionError}</span>
            </>
          ) : (
            <>
              <WifiOff size={14} />
              <span>Not connected</span>
            </>
          )}
        </div>
        <div className="banner-right">
          {connectionError && (
            <button className="banner-dismiss" onClick={() => setConnectionError(null)} title="Dismiss">
              <X size={14} />
            </button>
          )}
          {connectedSSID && (
            <button className="disconnect-btn" onClick={handleDisconnect}>
              <WifiOff size={16} />
              <span>Disconnect</span>
            </button>
          )}
        </div>
      </header>

      {/* ====== NEW TOP HEADER ====== */}
      <div className="top-header">
        <div className="brand">
          <div className="brand-icon">
            <Radio size={20} />
          </div>
          <span className="brand-name">LibreMP</span>
        </div>

        <div className="header-controls">
          <div className="search-bar">
            <Search size={18} />
            <input
              type="text"
              placeholder="Search networks..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>

          <div className="header-actions">
            <button className="icon-btn-rounded" onClick={scanNetworks} disabled={isScanning} title="Refresh">
              <RefreshCw size={20} className={isScanning ? 'spinning' : ''} />
            </button>
            <button className="icon-btn-rounded" onClick={() => setIsSettingsOpen(true)} title="Settings">
              <Settings size={20} />
            </button>
            <button className="theme-toggle" onClick={toggleTheme} title={theme === 'light' ? 'Switch to dark mode' : 'Switch to light mode'}>
              {theme === 'light' ? <Moon size={20} /> : <Sun size={20} />}
            </button>
          </div>
        </div>
      </div>

      <div className="app-body">
        {/* ====== MAIN CONTENT ====== */}
        <main className="main">
          {/* Scrollable Body */}
          <div className="main-scroll">
            {/* Available Networks */}
            <section className="networks-section">
              <h3 className="section-heading">
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
                  {filtered.map((n, idx) => {
                    const isConnected = n.ssid === connectedSSID;
                    const isConnecting = n.ssid === connectingSSID;
                    return (
                      <div
                        key={n.id}
                        className={`net-card ${isConnected ? 'connected' : ''} ${n.is_projector ? 'projector' : ''}`}
                      >
                        <div className="net-name">
                          <span>{n.name}</span>
                          <div className="net-icon">
                            {n.is_projector ? <MonitorDot size={20} /> : <Wifi size={20} />}
                          </div>
                        </div>
                        
                        <div className="net-meta">
                          {n.is_projector && <span className="projector-tag">Projector</span>}
                          <span>{n.security}</span>
                          <span className="meta-dot">·</span>
                          <span>{n.signal}% signal</span>
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
                          onClick={(e) => { e.stopPropagation(); isConnected ? handleDisconnect() : handleNetworkClick(n); }}
                          disabled={isConnecting}
                        >
                          {isConnecting ? (
                            <RotateCcw size={14} className="spinning" />
                          ) : isConnected ? (
                            <>
                              <WifiOff size={16} />
                              <span>Disconnect</span>
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
      <SettingsModal isOpen={isSettingsOpen} onClose={() => setIsSettingsOpen(false)} settings={appSettings} onApply={setAppSettings} />
      <ConnectionModeModal isOpen={isConnectionModeOpen} onClose={() => setIsConnectionModeOpen(false)} mode={connectionMode} setMode={setConnectionMode} />
      <HelpModal isOpen={isHelpOpen} onClose={() => setIsHelpOpen(false)} />
      <PasswordModal 
        isOpen={!!passwordModalNet} 
        networkName={passwordModalNet?.name || ''} 
        onCancel={() => setPasswordModalNet(null)}
        onSubmit={(pwd) => {
          if (passwordModalNet) handleConnect(passwordModalNet, pwd);
          setPasswordModalNet(null);
        }}
      />

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
