import { useState, useEffect, useCallback } from 'react';
import {
  HelpCircle, RefreshCw, Settings, Wifi, WifiOff, MonitorDot, Search,
  Signal, Radio, Zap, ArrowRight,
  Sun, Volume2, Gauge, Monitor, RotateCcw, AlertTriangle, X, Shield, ShieldCheck
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
}


function App() {
  const [networks, setNetworks] = useState<NetworkItem[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  const [connectionMode, setConnectionMode] = useState<'quick' | 'advanced'>('quick');

  // App settings (shared between sidebar + modal)
  const [appSettings, setAppSettings] = useState<AppSettings>(defaultSettings);

  // Connection state
  const [connectedSSID, setConnectedSSID] = useState<string | null>(null);
  const [connectingSSID, setConnectingSSID] = useState<string | null>(null);
  const [connectionError, setConnectionError] = useState<string | null>(null);

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
            });
          } else {
            // Mark existing Wi-Fi entry as a projector if it matches
            const idx = items.findIndex(n => n.name === p.name || n.ssid === p.name);
            if (idx >= 0) items[idx].is_projector = true;
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

      // Auto-reconnect logic without relying on recent array
      if (appSettings.autoReconnect && !connectedSSID) {
        // We only reconnect if the currently connected SSID on the OS has not dropped but we aren't tracking, or we need to poll os
      }
    } catch {
      // silent fail
    } finally {
      setIsScanning(false);
      isScanningRef.current = false;
    }
  }, [appSettings.autoReconnect, connectedSSID, networks.length]);

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
          {appSettings.autoReconnect && (
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
          <div className="brand" title="LibreMP">
            <div className="brand-icon">
              <Radio size={16} />
            </div>
            <div className="brand-text">
              <span className="brand-name">LibreMP</span>
              <span className="brand-ver">v1.0</span>
            </div>
          </div>

          <nav className="nav">
            <button className="nav-btn active">
              <Search size={16} />
              <span>Discovery</span>
              {networks.length > 0 && <span className="badge">{networks.length}</span>}
            </button>
          </nav>

          {/* Quick Settings — Interactive */}
          <div className="sidebar-section">
            <h4 className="section-label">Quick Settings</h4>
            <div className="quick-setting" onClick={() => setAppSettings((s: AppSettings) => ({ ...s, displayMode: s.displayMode === 'operations' ? 'movies' : 'operations' }))}>
              <Monitor size={14} />
              <span>Display</span>
              <span className="qs-value clickable">{appSettings.displayMode === 'operations' ? 'Operations' : 'Movies'}</span>
            </div>
            <div className="quick-setting">
              <Sun size={14} />
              <span>Brightness</span>
              
              {isEditingBrightness ? (
                <input
                  type="text"
                  autoFocus
                  className="qs-value-input"
                  value={brightnessInput}
                  onChange={e => setBrightnessInput(e.target.value.replace(/\D/g, ''))}
                  onKeyDown={e => {
                    if (e.key === 'Enter') {
                      let val = parseInt(brightnessInput);
                      if (!isNaN(val)) {
                        val = Math.max(10, Math.min(100, val));
                        setAppSettings((s: AppSettings) => ({ ...s, brightness: val }));
                      }
                      setIsEditingBrightness(false);
                    } else if (e.key === 'Escape') {
                      setIsEditingBrightness(false);
                    }
                  }}
                  onBlur={() => {
                    let val = parseInt(brightnessInput);
                    if (!isNaN(val)) {
                      val = Math.max(10, Math.min(100, val));
                      setAppSettings((s: AppSettings) => ({ ...s, brightness: val }));
                    }
                    setIsEditingBrightness(false);
                  }}
                />
              ) : (
                <span 
                  className="qs-value clickable" 
                  onClick={() => {
                    setBrightnessInput(appSettings.brightness.toString());
                    setIsEditingBrightness(true);
                  }}
                  title="Click to type"
                >
                  {appSettings.brightness}%
                </span>
              )}
            </div>
            <div className="quick-setting" onClick={() => setAppSettings((s: AppSettings) => ({ ...s, audioOutput: !s.audioOutput }))}>
              <Volume2 size={14} />
              <span>Audio</span>
              <span className={`qs-value clickable ${appSettings.audioOutput ? '' : 'off'}`}>{appSettings.audioOutput ? 'On' : 'Off'}</span>
            </div>
            <div className="quick-setting" onClick={() => setAppSettings((s: AppSettings) => ({ ...s, bandwidth: s.bandwidth === 15 ? 10 : s.bandwidth === 10 ? 5 : 15 }))}>
              <Gauge size={14} />
              <span>Bandwidth</span>
              <span className="qs-value clickable">{appSettings.bandwidth}Mbps</span>
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
                  {filtered.map((n, idx) => {
                    const isConnected = n.ssid === connectedSSID;
                    const isConnecting = n.ssid === connectingSSID;
                    return (
                      <div
                        key={n.id}
                        className={`net-card ${isConnected ? 'connected' : ''} ${n.is_projector ? 'projector' : ''}`}
                        style={{ animationDelay: `${idx * 40}ms` }}
                      >
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
                            <Shield size={10} />
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
