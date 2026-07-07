import { useState, useEffect, useCallback, useRef } from 'react';
import {
  HelpCircle, X, QrCode, KeyRound, Camera, Upload, Sun, Moon, SlidersHorizontal,
  RefreshCw, RotateCcw, Cast, Trash2, MonitorPlay, Radio,
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import './index.css';

import { SettingsModal, type AppSettings, defaultSettings } from './components/SettingsModal';
import { HelpModal } from './components/HelpModal';
import { PasswordModal } from './components/PasswordModal';
import { ManualConnectModal } from './components/ManualConnectModal';
import { LiveScanModal } from './components/LiveScanModal';

interface QrResult {
  ssid: string;
  password: string;
  ip: string;
}

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

interface SavedProjector {
  name: string;
  ssid: string;
  password: string;
  ip: string;
}

const projName = (ssid: string) => ssid.split('-')[0] || ssid;
const signalLevel = (s: number) => (s > 80 ? 5 : s > 60 ? 4 : s > 40 ? 3 : s > 20 ? 2 : 1);
const signalClass = (s: number) => (s > 60 ? 'high' : s > 30 ? 'mid' : 'low');

function App() {
  const [networks, setNetworks] = useState<NetworkItem[]>([]);
  const [saved, setSaved] = useState<SavedProjector[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  const [theme, setTheme] = useState<'light' | 'dark'>(() => {
    const stored = localStorage.getItem('libre-mp-theme');
    if (stored === 'light' || stored === 'dark') return stored;
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  });
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('libre-mp-theme', theme);
  }, [theme]);

  const [appSettings, setAppSettings] = useState<AppSettings>(() => {
    try {
      const raw = localStorage.getItem('libre-mp-settings');
      return raw ? { ...defaultSettings, ...JSON.parse(raw) } : defaultSettings;
    } catch {
      return defaultSettings;
    }
  });
  useEffect(() => {
    localStorage.setItem('libre-mp-settings', JSON.stringify(appSettings));
  }, [appSettings]);

  const [connectedSSID, setConnectedSSID] = useState<string | null>(null);
  const [connectingSSID, setConnectingSSID] = useState<string | null>(null);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [statusDetail, setStatusDetail] = useState<string | null>(null);
  const [isCasting, setIsCasting] = useState(false);
  const [castName, setCastName] = useState<string>('');

  const [toast, setToast] = useState<{ message: string; type: 'success' | 'info' | 'error' } | null>(null);

  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isHelpOpen, setIsHelpOpen] = useState(false);
  const [isManualOpen, setIsManualOpen] = useState(false);
  const [isLiveScanOpen, setIsLiveScanOpen] = useState(false);
  const [passwordModalNet, setPasswordModalNet] = useState<NetworkItem | null>(null);

  const uploadRef = useRef<HTMLInputElement>(null);
  const scanningRef = useRef(false);
  const autoReconnectTried = useRef(false);

  const notify = useCallback(
    (message: string, type: 'success' | 'info' | 'error' = 'info') => {
      if (appSettings.showNotifications) setToast({ message, type });
    },
    [appSettings.showNotifications]
  );

  // ---- data loading ----
  const loadSaved = useCallback(async () => {
    try {
      setSaved(await invoke<SavedProjector[]>('list_saved_projectors'));
    } catch {
      /* store may not exist yet */
    }
  }, []);

  const scanNetworks = useCallback(async () => {
    if (scanningRef.current) return;
    scanningRef.current = true;
    setIsScanning(true);
    try {
      const items: NetworkItem[] = [];
      try {
        const results = await invoke<WifiNetwork[]>('scan_wifi_networks');
        for (const n of results) {
          items.push({
            id: n.bssid || `wifi-${n.ssid}`,
            name: n.ssid || 'Hidden network',
            ssid: n.ssid,
            signal: n.signal,
            security: n.security,
            is_projector: n.is_projector,
          });
        }
      } catch { /* adapter may be busy */ }

      try {
        const projectors = await invoke<{ name: string; ip: string }[]>('discover_projectors');
        for (const p of projectors) {
          const existing = items.find((n) => n.name === p.name || n.ssid === p.name);
          if (existing) {
            existing.is_projector = true;
            existing.ip = p.ip;
          } else {
            items.push({
              id: `proj-${p.ip}`,
              name: p.name,
              ssid: p.name,
              signal: 100,
              security: 'Projector',
              is_projector: true,
              ip: p.ip,
            });
          }
        }
      } catch { /* no projectors on this LAN */ }

      if (items.length > 0) setNetworks(items);
    } finally {
      setIsScanning(false);
      scanningRef.current = false;
    }
  }, []);

  useEffect(() => {
    scanNetworks();
    loadSaved();
    const id = setInterval(scanNetworks, 12000);
    return () => clearInterval(id);
  }, [scanNetworks, loadSaved]);

  useEffect(() => {
    if (connectionError) {
      const t = setTimeout(() => setConnectionError(null), 8000);
      return () => clearTimeout(t);
    }
  }, [connectionError]);

  // ---- connection flow ----
  const startCasting = useCallback(
    async (name: string, ssid: string, password: string, ip: string) => {
      setStatusDetail('Starting cast…');
      await invoke('start_casting_async', { ssid, password });
      setIsCasting(true);
      setCastName(name);
      notify(`Casting to ${name}`, 'success');
      try {
        await invoke('save_projector', { name, ssid, password, ip: ip || '' });
        await loadSaved();
      } catch { /* persistence is best-effort */ }
    },
    [notify, loadSaved]
  );

  const connectProjector = useCallback(
    async (name: string, ssid: string, password: string, ip: string) => {
      setConnectingSSID(ssid);
      setConnectionError(null);
      setStatusDetail(`Joining ${name}…`);
      try {
        await new Promise((r) => setTimeout(r, 250));
        const ok = await invoke<boolean>('connect_to_wifi', { ssid, password });
        if (!ok) throw new Error('Could not join the network.');
        setConnectedSSID(ssid);
        await startCasting(name, ssid, password, ip);
        return true;
      } catch (err: any) {
        setConnectionError(typeof err === 'string' ? err : err?.message || 'Connection failed.');
        return false;
      } finally {
        setConnectingSSID(null);
        setStatusDetail(null);
      }
    },
    [startCasting]
  );

  const stopCasting = useCallback(async () => {
    try {
      await invoke('stop_casting');
    } catch { /* already stopped */ }
    setIsCasting(false);
    setCastName('');
    notify('Casting stopped.', 'info');
  }, [notify]);

  const disconnect = useCallback(async () => {
    if (isCasting) await stopCasting();
    setConnectedSSID(null);
  }, [isCasting, stopCasting]);

  const handleRowClick = (net: NetworkItem) => {
    const known = saved.find((s) => s.ssid === net.ssid);
    if (known) {
      connectProjector(known.name || projName(known.ssid), known.ssid, known.password, known.ip);
    } else if (!net.is_projector && net.security === 'Open') {
      connectProjector(net.name, net.ssid, '', net.ip || '');
    } else {
      // Projectors and secured networks need a key (the projector's MAC for Epson).
      setPasswordModalNet(net);
    }
  };

  const handleQrDecoded = useCallback(
    (res: QrResult) => {
      notify(`Found ${res.ssid}`, 'info');
      connectProjector(projName(res.ssid), res.ssid, res.password, res.ip);
    },
    [connectProjector, notify]
  );

  const handleUpload = async (file: File) => {
    try {
      setStatusDetail('Reading QR…');
      const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
      const res = await invoke<QrResult>('decode_projector_qr', { imageBytes: bytes });
      setStatusDetail(null);
      handleQrDecoded(res);
    } catch (err: any) {
      setStatusDetail(null);
      setConnectionError(typeof err === 'string' ? err : 'Could not read the QR code.');
    }
  };

  const forgetSaved = async (ssid: string) => {
    try {
      await invoke('forget_projector', { ssid });
      await loadSaved();
    } catch { /* ignore */ }
  };

  // Auto-reconnect to the most recent projector once, if enabled.
  useEffect(() => {
    if (autoReconnectTried.current || !appSettings.autoReconnect || saved.length === 0) return;
    autoReconnectTried.current = true;
    const p = saved[0];
    connectProjector(p.name || projName(p.ssid), p.ssid, p.password, p.ip);
  }, [appSettings.autoReconnect, saved, connectProjector]);

  // ---- derived ----
  const savedSsids = new Set(saved.map((s) => s.ssid));
  const available = networks
    .filter((n) => n.name.toLowerCase().includes(searchQuery.toLowerCase()))
    .filter((n) => !savedSsids.has(n.ssid))
    .sort((a, b) => {
      if (a.is_projector !== b.is_projector) return a.is_projector ? -1 : 1;
      return b.signal - a.signal;
    });

  const lampState = isCasting ? 'is-casting' : connectedSSID ? 'is-connected' : '';
  const lampLabel = isCasting ? 'Casting' : connectedSSID ? 'Connected' : 'Idle';

  return (
    <div className="lm-app">
      <header className="lm-topbar">
        <div className="lm-brand">
          <span className="lm-brand-mark">Libre<b>MP</b></span>
        </div>
        <div className="lm-topbar-spacer" />

        <div className={`lm-lamp ${lampState}`} title={lampLabel}>
          <span className="lm-lamp-dot" />
          {lampLabel}
        </div>

        <div className="lm-search">
          <Radio size={15} />
          <input
            placeholder="Filter networks"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>

        <button className="lm-iconbtn" onClick={scanNetworks} title="Rescan" aria-label="Rescan">
          <RefreshCw size={17} className={isScanning ? 'lm-spin' : ''} />
        </button>
        <button
          className="lm-iconbtn"
          onClick={() => setTheme((t) => (t === 'light' ? 'dark' : 'light'))}
          title="Toggle theme"
          aria-label="Toggle theme"
        >
          {theme === 'light' ? <Moon size={17} /> : <Sun size={17} />}
        </button>
        <button className="lm-iconbtn" onClick={() => setIsSettingsOpen(true)} title="Settings" aria-label="Settings">
          <SlidersHorizontal size={17} />
        </button>
        <button className="lm-iconbtn" onClick={() => setIsHelpOpen(true)} title="Help" aria-label="Help">
          <HelpCircle size={17} />
        </button>
      </header>

      {(connectingSSID || connectionError) && (
        <div className={`lm-banner ${connectionError ? 'error' : 'connecting'}`}>
          {connectionError ? (
            <>
              <X size={15} />
              <span>{connectionError}</span>
              <button className="lm-iconbtn" onClick={() => setConnectionError(null)} style={{ marginLeft: 'auto', width: 28, height: 28 }} aria-label="Dismiss">
                <X size={14} />
              </button>
            </>
          ) : (
            <>
              <RotateCcw size={15} className="lm-spin" />
              <span>{statusDetail || `Connecting to ${connectingSSID}…`}</span>
            </>
          )}
        </div>
      )}

      <main className="lm-body">
        <div className="lm-wrap">
          {/* HERO: connect via QR */}
          <section className="lm-section">
            <p className="lm-eyebrow">
              Connect a projector <span className="lm-rule" />
            </p>
            <div className="lm-hero">
              <button className="lm-hero-card" onClick={() => setIsLiveScanOpen(true)}>
                <span className="lm-hero-icon"><Camera size={20} /></span>
                <span className="lm-hero-title">Live scan</span>
                <span className="lm-hero-sub">
                  Point your camera at the QR on the projector’s LAN screen. LibreMP reads it
                  live and connects — no typing.
                </span>
              </button>
              <button className="lm-hero-card" onClick={() => uploadRef.current?.click()}>
                <span className="lm-hero-icon"><Upload size={20} /></span>
                <span className="lm-hero-title">Upload QR photo</span>
                <span className="lm-hero-sub">
                  Already have a picture of the projector’s QR? Choose the image and LibreMP
                  does the rest.
                </span>
              </button>
              <button className="lm-hero-card lm-hero-lamp" onClick={() => setIsManualOpen(true)}>
                <span className="lm-hero-icon"><KeyRound size={20} /></span>
                <span className="lm-hero-title">Enter details</span>
                <span className="lm-hero-sub">
                  No QR? On the projector’s network screen, read its SSID and passphrase and
                  type them here.
                </span>
              </button>
            </div>
          </section>

          {/* SAVED */}
          {saved.length > 0 && (
            <section className="lm-section">
              <p className="lm-eyebrow">
                Saved <span className="lm-count">{saved.length}</span> <span className="lm-rule" />
              </p>
              <div className="lm-cards">
                {saved.map((p) => {
                  const isConn = p.ssid === connectedSSID;
                  const isConnecting = p.ssid === connectingSSID;
                  return (
                    <div key={p.ssid} className={`lm-row is-projector ${isConn ? 'is-connected' : ''}`}>
                      <MonitorPlay size={18} style={{ color: 'var(--lm-signal)', flexShrink: 0 }} />
                      <div className="lm-row-main">
                        <div className="lm-row-name">{p.name || projName(p.ssid)}</div>
                        <div className="lm-row-meta">{p.ip ? `${p.ip} · ` : ''}{p.ssid}</div>
                      </div>
                      {isConn && isCasting ? (
                        <button className="lm-btn danger" onClick={stopCasting}>Stop cast</button>
                      ) : (
                        <button
                          className="lm-btn signal"
                          disabled={isConnecting}
                          onClick={() => connectProjector(p.name || projName(p.ssid), p.ssid, p.password, p.ip)}
                        >
                          {isConnecting ? <RotateCcw size={14} className="lm-spin" /> : <><Cast size={14} /> Reconnect</>}
                        </button>
                      )}
                      <button className="lm-iconbtn" onClick={() => forgetSaved(p.ssid)} title="Forget" aria-label="Forget projector">
                        <Trash2 size={16} />
                      </button>
                    </div>
                  );
                })}
              </div>
            </section>
          )}

          {/* AVAILABLE */}
          <section className="lm-section">
            <p className="lm-eyebrow">
              Available <span className="lm-count">{available.length}</span> <span className="lm-rule" />
            </p>
            {available.length === 0 ? (
              <div className="lm-empty">
                <span className="lm-empty-icon"><QrCode size={26} /></span>
                <h4>{isScanning ? 'Scanning…' : 'Nothing here yet'}</h4>
                <p>Scan the projector’s QR above, or check your Wi-Fi adapter and rescan.</p>
              </div>
            ) : (
              <div className="lm-cards">
                {available.map((n) => {
                  const isConn = n.ssid === connectedSSID;
                  const isConnecting = n.ssid === connectingSSID;
                  return (
                    <div key={n.id} className={`lm-row ${n.is_projector ? 'is-projector' : ''} ${isConn ? 'is-connected' : ''}`}>
                      <div className="lm-row-main">
                        <div className="lm-row-name">{n.name}</div>
                        <div className="lm-row-meta">{n.ip ? `${n.ip} · ` : ''}{n.is_projector ? 'Projector' : n.security}</div>
                      </div>

                      {!n.is_projector && (
                        <div className={`lm-bars ${signalClass(n.signal)}`}>
                          {[1, 2, 3, 4, 5].map((i) => (
                            <span key={i} className={`b ${i <= signalLevel(n.signal) ? 'on' : ''}`} />
                          ))}
                        </div>
                      )}
                      <span className={`lm-pill ${n.is_projector ? 'proj' : ''} ${isConn ? 'on' : ''}`}>
                        {isConn ? 'Connected' : n.is_projector ? 'Projector' : `${n.signal}%`}
                      </span>

                      {isConn && isCasting ? (
                        <button className="lm-btn danger" onClick={stopCasting}>Stop cast</button>
                      ) : isConn ? (
                        <button className="lm-btn ghost" onClick={disconnect}>Disconnect</button>
                      ) : (
                        <button
                          className={`lm-btn ${n.is_projector ? 'signal' : ''}`}
                          disabled={isConnecting}
                          onClick={() => handleRowClick(n)}
                        >
                          {isConnecting ? <RotateCcw size={14} className="lm-spin" /> : 'Connect'}
                        </button>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </section>
        </div>
      </main>

      {/* casting bar */}
      {isCasting && (
        <div className="lm-castbar">
          <span className="lm-lamp-dot" />
          <div className="lm-castbar-text">
            <strong>Casting to {castName || 'projector'}</strong>
            <span>{connectedSSID}</span>
          </div>
          <button className="lm-btn danger" onClick={stopCasting}>Stop</button>
        </div>
      )}

      {/* hidden upload input */}
      <input
        ref={uploadRef}
        type="file"
        accept="image/*"
        style={{ display: 'none' }}
        onChange={(e) => {
          const f = e.target.files?.[0];
          if (f) handleUpload(f);
          e.currentTarget.value = '';
        }}
      />

      {/* modals */}
      <SettingsModal isOpen={isSettingsOpen} onClose={() => setIsSettingsOpen(false)} settings={appSettings} onApply={setAppSettings} />
      <HelpModal isOpen={isHelpOpen} onClose={() => setIsHelpOpen(false)} />
      <LiveScanModal
        isOpen={isLiveScanOpen}
        onClose={() => setIsLiveScanOpen(false)}
        onDecoded={(r) => {
          setIsLiveScanOpen(false);
          handleQrDecoded(r);
        }}
      />
      <ManualConnectModal
        isOpen={isManualOpen}
        onClose={() => setIsManualOpen(false)}
        onConnect={(ssid, password) => {
          setIsManualOpen(false);
          connectProjector(projName(ssid), ssid, password, '');
        }}
      />
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
          const net = passwordModalNet;
          if (!net) return;
          connectProjector(net.name, net.ssid, pwd, net.ip || '').then((ok) => {
            if (ok) setPasswordModalNet(null);
          });
        }}
      />

      {toast && (
        <div className={`lm-toast ${toast.type === 'error' ? 'err' : ''}`}>
          <span>{toast.message}</span>
          <button className="lm-iconbtn" style={{ width: 26, height: 26 }} onClick={() => setToast(null)} aria-label="Dismiss">
            <X size={14} />
          </button>
        </div>
      )}
    </div>
  );
}

export default App;
