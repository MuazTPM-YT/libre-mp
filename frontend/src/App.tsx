import { useState, useEffect, useCallback } from 'react';
import {
  HelpCircle, RefreshCw, Settings, Search,
  Signal, Radio,
  Sun, Moon, RotateCcw, X
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

  // Toast notification state
  const [toast, setToast] = useState<{ message: string; type: 'success' | 'info' } | null>(null);

  // UI state
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isConnectionModeOpen, setIsConnectionModeOpen] = useState(false);
  const [isHelpOpen, setIsHelpOpen] = useState(false);
  const [passwordModalNet, setPasswordModalNet] = useState<NetworkItem | null>(null);
  
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
        setToast({ message: `Connected to ${network.name}`, type: 'success' });

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
    const name = connectedNetwork?.name || connectedSSID || 'network';
    setConnectedSSID(null);
    setConnectionError(null);
    setToast({ message: `Disconnected from ${name}`, type: 'info' });
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
            <span>Connected to <strong>{connectedNetwork?.name || connectedSSID}</strong></span>
          ) : connectingSSID ? (
            <span>Connecting to <strong>{connectingSSID}</strong>...</span>
          ) : connectionError ? (
            <span className="banner-error-text">{connectionError}</span>
          ) : (
            <span>Not connected</span>
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
                <div className="network-table">
                  {/* Column Header */}
                  <div className="net-table-header">
                    <span className="col-status">Status</span>
                    <span className="col-name">Projector Name</span>
                    <span className="col-ssid">SSID</span>
                    <span className="col-signal">Signal</span>
                    <span className="col-action"></span>
                  </div>

                  {/* Rows */}
                  <div className="network-list">
                    {filtered.map((n) => {
                      const isConnected = n.ssid === connectedSSID;
                      const isConnecting = n.ssid === connectingSSID;
                      return (
                        <div
                          key={n.id}
                          className={`net-row ${isConnected ? 'connected' : ''} ${n.is_projector ? 'projector' : ''}`}
                        >
                          {/* Status */}
                          <div className="col-status">
                            <span className={`status-label ${isConnected ? 'on' : ''}`}>
                              {isConnected ? 'Connected' : isConnecting ? 'Connecting…' : n.is_projector ? 'Available' : n.security}
                            </span>
                          </div>

                          {/* Projector Name */}
                          <span className="col-name" title={n.name}>{n.name}</span>

                          {/* SSID */}
                          <span className="col-ssid" title={n.ssid}>{n.ssid}</span>

                          {/* Signal */}
                          <div className="col-signal">
                            <div className={`signal-bars ${signalColor(n.signal)}`}>
                              {[1,2,3,4,5].map(i => (
                                <div key={i} className={`bar ${i <= signalLevel(n.signal) ? 'on' : ''}`} />
                              ))}
                            </div>
                          </div>

                          {/* Action */}
                          <div className="col-action">
                            <button
                              className={`connect-btn-sm ${isConnected ? 'active' : ''}`}
                              onClick={(e) => { e.stopPropagation(); isConnected ? handleDisconnect() : handleNetworkClick(n); }}
                              disabled={isConnecting}
                            >
                              {isConnecting ? (
                                <RotateCcw size={14} className="spinning" />
                              ) : isConnected ? (
                                'Disconnect'
                              ) : (
                                'Connect'
                              )}
                            </button>
                          </div>
                        </div>
                      );
                    })}
                  </div>
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

      {/* ====== TOAST POPUP ====== */}
      {toast && (
        <div className={`toast toast-${toast.type}`}>
          <span>{toast.message}</span>
          <button className="toast-close" onClick={() => setToast(null)}>
            <X size={14} />
          </button>
        </div>
      )}
    </div>
  );
}

export default App;
