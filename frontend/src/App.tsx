import { useState, useEffect, useCallback } from 'react';
import { HelpCircle, X } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import './index.css';

import { SettingsModal, type AppSettings, defaultSettings } from './components/SettingsModal';
import { ConnectionModeModal } from './components/ConnectionModeModal';
import { HelpModal } from './components/HelpModal';
import { PasswordModal } from './components/PasswordModal';
import { StatusBanner } from './components/StatusBanner';
import { TopHeader } from './components/TopHeader';
import { NetworkTable } from './components/NetworkTable';
import { OSSelectModal } from './components/OSSelectModal';

export interface NetworkItem {
  id: string;
  name: string;
  ssid: string;
  signal: number;
  security: string;
  is_projector: boolean;
  ip?: string;
}

interface WifiNetwork {
  ssid: string;
  bssid: string;
  signal: number;
  security: string;
  is_projector: boolean;
}

/** Main application orchestrator managing network state and UI modals */
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

  // App settings
  const [appSettings, setAppSettings] = useState<AppSettings>(defaultSettings);

  // Connection state
  const [connectedSSID, setConnectedSSID] = useState<string | null>(null);
  const [connectedPassword, setConnectedPassword] = useState<string>('');
  const [connectingSSID, setConnectingSSID] = useState<string | null>(null);
  const [connectionStatusDetail, setConnectionStatusDetail] = useState<string | null>(null);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [isCasting, setIsCasting] = useState<boolean>(false);

  // Toast notification state
  const [toast, setToast] = useState<{ message: string; type: 'success' | 'info' } | null>(null);

  // UI state
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isConnectionModeOpen, setIsConnectionModeOpen] = useState(false);
  const [isHelpOpen, setIsHelpOpen] = useState(false);
  const [isOSSelectOpen, setIsOSSelectOpen] = useState(false);
  const [passwordModalNet, setPasswordModalNet] = useState<NetworkItem | null>(null);

  const isScanningRef = { current: false };

  const scanNetworks = useCallback(async () => {
    if (isScanningRef.current) return;
    isScanningRef.current = true;
    try {
      setIsScanning(true);
      let items: NetworkItem[] = [];

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

      try {
        const projectors: any[] = await invoke('discover_projectors');
        for (const p of projectors) {
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

  useEffect(() => {
    scanNetworks();
    const interval = setInterval(scanNetworks, 12000);
    return () => clearInterval(interval);
  }, [scanNetworks]);

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
    setConnectionStatusDetail(`Initializing connection to ${network.name}...`);

    try {
      // Small delay for smoother transition
      await new Promise(r => setTimeout(r, 400));
      setConnectionStatusDetail(`Authenticating with ${network.name}...`);

      const success: boolean = await invoke('connect_to_wifi', { ssid: network.ssid, password });
      if (success) {
        setConnectedSSID(network.ssid);
        setConnectedPassword(password || '');

        await new Promise(r => setTimeout(r, 200));

        // Show OS selection modal after successful connection
        setIsOSSelectOpen(true);

        return true;
      }
      return false;
    } catch (err: any) {
      console.error("Connection failed:", err);
      const msg = typeof err === 'string' ? err : (err?.message || 'Connection failed. Please try again.');
      setConnectionError(msg);
      return false;
    } finally {
      setConnectingSSID(null);
      setConnectionStatusDetail(null);
    }
  };

  const handleDisconnect = () => {
    if (isCasting) {
      handleStopCast();
    }
    setConnectedSSID(null);
    setConnectionError(null);
  };

  const handleOSSelect = async (osMode: number) => {
    setIsOSSelectOpen(false);
    if (!connectedSSID) return;

    try {
      setConnectionStatusDetail('Starting stream...');
      await invoke('start_casting_async', {
        ssid: connectedSSID,
        password: connectedPassword,
        osMode,
      });
      setIsCasting(true);
      setConnectionStatusDetail(null);
      setToast({ message: 'Screen casting started!', type: 'success' });
    } catch (err: any) {
      console.error('Failed to start cast:', err);
      setConnectionError(typeof err === 'string' ? err : 'Failed to start streaming');
      setConnectionStatusDetail(null);
    }
  };

  const handleStopCast = async () => {
    try {
      await invoke('stop_casting');
      setIsCasting(false);
      setToast({ message: 'Screen casting stopped.', type: 'info' });
    } catch (err: any) {
      console.error('Failed to stop cast:', err);
    }
  };

  const filtered = networks
    .filter(n => n.name.toLowerCase().includes(searchQuery.toLowerCase()))
    .sort((a, b) => {
      // 1. Connected network first
      const aConn = a.ssid === connectedSSID;
      const bConn = b.ssid === connectedSSID;
      if (aConn && !bConn) return -1;
      if (!aConn && bConn) return 1;

      // 2. Projectors next
      if (a.is_projector && !b.is_projector) return -1;
      if (!a.is_projector && b.is_projector) return 1;

      // 3. Then signal strength
      return b.signal - a.signal;
    });

  return (
    <div className="app">
      <StatusBanner
        connectedSSID={connectedSSID}
        connectingSSID={connectingSSID}
        connectionError={connectionError}
        statusDetail={connectionStatusDetail}
        onDismissError={() => setConnectionError(null)}
      />

      <TopHeader
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        onRefresh={scanNetworks}
        onOpenSettings={() => setIsSettingsOpen(true)}
        onToggleTheme={toggleTheme}
        theme={theme}
        isScanning={isScanning}
      />

      <div className="app-body">
        <main className="main">
          <div className="main-scroll">
            <NetworkTable
              networks={filtered}
              connectedSSID={connectedSSID}
              connectingSSID={connectingSSID}
              onConnect={handleNetworkClick}
              onDisconnect={handleDisconnect}
              isScanning={isScanning}
              isCasting={isCasting}
              onStartCast={() => setIsOSSelectOpen(true)}
              onStopCast={handleStopCast}
            />
          </div>
        </main>
      </div>

      <SettingsModal isOpen={isSettingsOpen} onClose={() => setIsSettingsOpen(false)} settings={appSettings} onApply={setAppSettings} />
      <ConnectionModeModal isOpen={isConnectionModeOpen} onClose={() => setIsConnectionModeOpen(false)} mode={connectionMode} setMode={setConnectionMode} />
      <HelpModal isOpen={isHelpOpen} onClose={() => setIsHelpOpen(false)} />
      <OSSelectModal isOpen={isOSSelectOpen} onCancel={() => setIsOSSelectOpen(false)} onSelect={handleOSSelect} />

      <PasswordModal
        isOpen={!!passwordModalNet}
        networkName={passwordModalNet?.name || ''}
        isLoading={connectingSSID === passwordModalNet?.ssid}
        error={connectingSSID === passwordModalNet?.ssid ? null : connectionError}
        onCancel={() => {
          setPasswordModalNet(null);
          setConnectionError(null);
        }}
        onSubmit={(pwd) => {
          if (passwordModalNet) {
            handleConnect(passwordModalNet, pwd).then((success) => {
              if (success) {
                setPasswordModalNet(null);
              }
            });
          }
        }}
      />

      {!isHelpOpen && (
        <button className="help-fab" onClick={() => setIsHelpOpen(true)} title="Help &amp; Guide">
          <HelpCircle size={20} />
        </button>
      )}

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
