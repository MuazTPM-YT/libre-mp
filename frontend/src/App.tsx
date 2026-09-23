import { useCallback, useEffect, useRef, useState } from 'react';
import {
  Camera, ChevronRight, CircleAlert, CircleHelp, LoaderCircle, Lock, Projector, RefreshCw, Router, Search, Settings,
  Trash2, X,
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import './index.css';

import { SettingsModal, type AppSettings, defaultSettings } from './components/SettingsModal';
import { Toast, type ToastData } from './components/Toast';
import { HelpModal } from './components/HelpModal';
import { PasswordModal } from './components/PasswordModal';
import { ManualConnectModal } from './components/ManualConnectModal';
import { LiveScanModal, type QrResult } from './components/LiveScanModal';
import { KeywordModal } from './components/KeywordModal';

interface WifiNetwork {
  ssid: string;
  bssid: string;
  signal: number;
  security: string;
  is_projector: boolean;
}

interface FoundProjector {
  name: string;
  ip: string;
}

interface SavedProjector {
  name: string;
  ssid: string;
  ip: string;
}

type CastEvent =
  | { id: number; phase: 'sharing' }
  | { id: number; phase: 'connecting'; attempt: number }
  | { id: number; phase: 'casting'; projector: string }
  | { id: number; phase: 'reconnecting'; reason: string };

interface CastEnd {
  id: number;
  error: { kind: 'capture' | 'rejected' | 'unreachable'; message: string } | null;
}

// everything one connect needs
interface Target {
  name: string;
  ssid: string;
  password: string;
  ip: string;
  joinWifi: boolean;
  keyword?: string;
}

type Status =
  | { kind: 'idle' }
  | { kind: 'working'; target: Target; detail: string; cancellable: boolean }
  | { kind: 'casting'; target: Target }
  | { kind: 'error'; message: string; target?: Target; rejected?: boolean };

// one list row
interface Row {
  key: string;
  name: string;
  sub: string;
  projector: boolean;
  signal?: number;
  secured?: boolean;
  network?: WifiNetwork;
  found?: FoundProjector;
}

const SETTINGS_KEY = 'libre-mp-settings';
const OTHERS_KEY = 'libre-mp-show-other-networks';
const SCAN_EVERY_MS = 12000;

// epson ssid = <name>-<random suffix>; name may hold '-'
const projName = (ssid: string) => (ssid.includes('-') ? ssid.slice(0, ssid.lastIndexOf('-')) : ssid);
const errText = (e: unknown) => (typeof e === 'string' ? e : e instanceof Error ? e.message : 'Something went wrong.');
const matches = (q: string, ...s: string[]) => !q || s.some((x) => x.toLowerCase().includes(q.toLowerCase()));

// storage can throw (private mode, blocked site data)
function readStore<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? { ...fallback, ...JSON.parse(raw) } : fallback;
  } catch {
    return fallback;
  }
}
function writeStore(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    /* per-viewer nicety only */
  }
}

// wi-fi strength glyph: full fan, unlit arcs dimmed like macos
function SignalIcon({ signal }: { signal: number }) {
  const level = signal > 70 ? 3 : signal > 45 ? 2 : signal > 20 ? 1 : 0;
  const lit = (n: number) => ({ opacity: level >= n ? 1 : 0.3 });
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" role="img" aria-label={`Signal ${signal}%`}>
      <path d="M12 20h.01" />
      <path d="M8.5 16.43a5 5 0 0 1 7 0" style={lit(1)} />
      <path d="M5 12.86a10 10 0 0 1 14 0" style={lit(2)} />
      <path d="M2 8.82a15 15 0 0 1 20 0" style={lit(3)} />
    </svg>
  );
}

// cast phase words for a person
function phaseText(e: CastEvent, name: string): string {
  switch (e.phase) {
    case 'sharing':
      return 'Waiting for screen sharing. Allow it if your desktop asks.';
    case 'connecting':
      return e.attempt > 1 ? `Trying ${name} again…` : `Connecting to ${name}…`;
    case 'reconnecting':
      return 'The connection dropped. Reconnecting…';
    default:
      return '';
  }
}

// main window: status card, saved, projectors, other networks
function App() {
  const [settings, setSettings] = useState<AppSettings>(() => readStore(SETTINGS_KEY, defaultSettings));
  const [status, setStatus] = useState<Status>({ kind: 'idle' });
  const [networks, setNetworks] = useState<WifiNetwork[]>([]);
  const [found, setFound] = useState<FoundProjector[]>([]);
  const [saved, setSaved] = useState<SavedProjector[]>([]);
  const [scanning, setScanning] = useState(false);
  const [query, setQuery] = useState('');
  const [showOthers, setShowOthers] = useState(() => readStore(OTHERS_KEY, { open: false }).open);
  const [toast, setToast] = useState<ToastData | null>(null);

  const [sheet, setSheet] = useState<null | 'settings' | 'help' | 'manual' | 'scan' | 'keyword'>(null);
  const [passwordFor, setPasswordFor] = useState<Target | null>(null);
  const [passwordError, setPasswordError] = useState<string | null>(null);

  const uploadRef = useRef<HTMLInputElement>(null);
  const scanningRef = useRef(false);
  const runRef = useRef(0);
  const castIdRef = useRef<number | null>(null);
  const castTargetRef = useRef<Target | null>(null);
  const autoTried = useRef(false);

  const busy = status.kind === 'working' || status.kind === 'casting';
  // no os title bar (linux): toolbar drags window and has close
  const [ownChrome, setOwnChrome] = useState(false);

  useEffect(() => {
    getCurrentWindow()
      .isDecorated()
      .then((decorated) => setOwnChrome(!decorated))
      .catch(() => {});
  }, []);

  // drag window by toolbar blank space; single press only, so no double-click maximize
  const dragWindow = (e: React.MouseEvent) => {
    if (!ownChrome || e.button !== 0 || e.detail > 1) return;
    if ((e.target as HTMLElement).closest('button, input, label')) return;
    getCurrentWindow().startDragging().catch(() => {});
  };

  // appearance: follow os unless forced
  useEffect(() => {
    const root = document.documentElement;
    if (settings.appearance === 'system') root.removeAttribute('data-theme');
    else root.setAttribute('data-theme', settings.appearance);
    writeStore(SETTINGS_KEY, settings);
  }, [settings]);

  useEffect(() => writeStore(OTHERS_KEY, { open: showOthers }), [showOthers]);

  const notify = useCallback(
    (message: string, type: ToastData['type'] = 'info') => {
      if (settings.showNotifications || type === 'error') setToast({ message, type });
    },
    [settings.showNotifications],
  );
  const dismissToast = useCallback(() => setToast(null), []);

  const loadSaved = useCallback(async () => {
    try {
      setSaved(await invoke<SavedProjector[]>('list_saved_projectors'));
    } catch {
      /* no store yet */
    }
  }, []);

  // wi-fi list + lan discovery, one at a time
  const scan = useCallback(async () => {
    if (scanningRef.current) return;
    scanningRef.current = true;
    setScanning(true);
    try {
      const [wifi, lan] = await Promise.allSettled([
        invoke<WifiNetwork[]>('scan_wifi_networks'),
        invoke<FoundProjector[]>('discover_projectors'),
      ]);
      if (wifi.status === 'fulfilled') setNetworks(wifi.value);
      if (lan.status === 'fulfilled') setFound(lan.value);
    } finally {
      scanningRef.current = false;
      setScanning(false);
    }
  }, []);

  useEffect(() => {
    loadSaved();
  }, [loadSaved]);

  // rescan on a timer, paused while connecting or casting
  useEffect(() => {
    if (busy) return;
    scan();
    const id = window.setInterval(scan, SCAN_EVERY_MS);
    return () => window.clearInterval(id);
  }, [busy, scan]);

  // remember projector once casting really works; password goes to keychain
  const remember = useCallback(
    async (t: Target, reportedName: string) => {
      try {
        await invoke('save_projector', { name: reportedName || t.name, ssid: t.ssid, password: t.password, ip: t.ip });
      } catch (e) {
        notify(errText(e), 'error');
      }
      loadSaved();
    },
    [loadSaved, notify],
  );

  // cast thread events. ids from an older cast get ignored
  useEffect(() => {
    const offEvent = listen<CastEvent>('cast-event', ({ payload }) => {
      const t = castTargetRef.current;
      if (payload.id !== castIdRef.current || !t) return;
      if (payload.phase === 'casting') {
        setStatus({ kind: 'casting', target: t });
        remember(t, payload.projector);
      } else {
        setStatus({ kind: 'working', target: t, detail: phaseText(payload, t.name), cancellable: true });
      }
    });
    const offEnd = listen<CastEnd>('cast-end', ({ payload }) => {
      if (payload.id !== castIdRef.current) return;
      const t = castTargetRef.current ?? undefined;
      castIdRef.current = null;
      castTargetRef.current = null;
      invoke('stop_cast').catch(() => {});
      if (payload.error) {
        setStatus({ kind: 'error', message: payload.error.message, target: t, rejected: payload.error.kind === 'rejected' });
      } else {
        setStatus({ kind: 'idle' });
      }
    });
    return () => {
      offEvent.then((f) => f());
      offEnd.then((f) => f());
    };
  }, [remember]);

  // find projector, join its wi-fi if needed, start cast
  const connect = useCallback(async (t: Target): Promise<boolean> => {
    const run = ++runRef.current;
    const stale = () => runRef.current !== run;
    setStatus({ kind: 'working', target: t, detail: `Looking for ${t.name}…`, cancellable: true });
    try {
      const reachable = !!t.ip && (await invoke<boolean>('probe_projector', { ip: t.ip }));
      if (stale()) return false;
      if (!reachable) {
        if (!t.joinWifi) throw new Error(`${t.name} is not answering at ${t.ip}. Check that it is on and on this network.`);
        // joining cannot be cancelled halfway, or the os may end up on the wrong network
        setStatus({ kind: 'working', target: t, detail: `Joining ${t.name}’s Wi-Fi…`, cancellable: false });
        await invoke('connect_to_wifi', { ssid: t.ssid, password: t.password || null });
        if (stale()) return false;
      }
      const id = (Date.now() % 1e9) + run;
      castIdRef.current = id;
      castTargetRef.current = t;
      setStatus({ kind: 'working', target: t, detail: 'Starting…', cancellable: true });
      await invoke('start_cast', { castId: id, ssid: t.ssid, password: t.password, ip: t.ip || null, keyword: t.keyword ?? null });
      return true;
    } catch (e) {
      if (stale()) return false;
      castIdRef.current = null;
      castTargetRef.current = null;
      setStatus({ kind: 'error', message: errText(e), target: t });
      invoke('stop_cast').catch(() => {});
      return false;
    }
  }, []);

  // user stop or cancel: forget cast id first so its end event is ignored
  const stop = useCallback(async () => {
    const wasCasting = status.kind === 'casting';
    runRef.current++;
    castIdRef.current = null;
    castTargetRef.current = null;
    setStatus({ kind: 'idle' });
    await invoke('stop_cast').catch(() => {});
    if (wasCasting) notify('Casting stopped.');
  }, [status.kind, notify]);

  // row click: pick right path for lan projector, saved, open, or secured network
  const connectRow = (row: Row) => {
    if (row.found) {
      connect({ name: row.found.name, ssid: '', password: '', ip: row.found.ip, joinWifi: false });
      return;
    }
    const net = row.network!;
    const name = net.is_projector ? projName(net.ssid) : net.ssid;
    const target: Target = { name, ssid: net.ssid, password: '', ip: '', joinWifi: true };
    if (net.security === 'Open') connect(target);
    else {
      setPasswordError(null);
      setPasswordFor(target);
    }
  };

  const connectSaved = (p: SavedProjector) =>
    connect({ name: p.name || projName(p.ssid), ssid: p.ssid, password: '', ip: p.ip, joinWifi: !!p.ssid });

  const forget = async (p: SavedProjector) => {
    try {
      await invoke('forget_projector', { key: p.ssid || p.name });
    } catch (e) {
      notify(errText(e), 'error');
    }
    loadSaved();
  };

  const fromQr = useCallback(
    (qr: QrResult) => {
      setSheet(null);
      connect({ name: projName(qr.ssid), ssid: qr.ssid, password: qr.password, ip: qr.ip, joinWifi: true });
    },
    [connect],
  );

  const upload = async (file: File) => {
    try {
      const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
      fromQr(await invoke<QrResult>('decode_projector_qr', { imageBytes: bytes }));
    } catch (e) {
      setStatus({ kind: 'error', message: errText(e) });
    }
  };

  // rejoin last projector once at launch, if asked
  useEffect(() => {
    if (autoTried.current || !settings.autoReconnect || saved.length === 0) return;
    autoTried.current = true;
    connectSaved(saved[0]);
  }, [settings.autoReconnect, saved]);

  // list rows, saved ones not repeated
  const savedKeys = new Set(saved.flatMap((p) => [p.ssid, p.name, p.ip]).filter(Boolean));
  const projectorRows: Row[] = [
    ...found
      .filter((f) => !savedKeys.has(f.name) && !savedKeys.has(f.ip))
      .map((f) => ({ key: `lan-${f.ip}`, name: f.name, sub: `On this network · ${f.ip}`, projector: true, found: f })),
    ...networks
      .filter((n) => n.is_projector && !savedKeys.has(n.ssid))
      .map((n) => ({
        key: `wifi-${n.ssid}`,
        name: projName(n.ssid),
        sub: 'Projector Wi-Fi',
        projector: true,
        signal: n.signal,
        secured: n.security !== 'Open',
        network: n,
      })),
  ].filter((r) => matches(query, r.name, r.sub));
  const otherRows: Row[] = networks
    .filter((n) => !n.is_projector && !savedKeys.has(n.ssid) && matches(query, n.ssid))
    .map((n) => ({
      key: `wifi-${n.ssid}`,
      name: n.ssid,
      sub: n.security === 'Open' ? 'Open network' : 'Secured network',
      projector: false,
      signal: n.signal,
      secured: n.security !== 'Open',
      network: n,
    }));
  const savedRows = saved.filter((p) => matches(query, p.name, p.ssid));

  const connectButton = (onClick: () => void, label: string) => (
    <button type="button" className="lm-btn" disabled={busy} onClick={onClick} aria-label={label}>
      Connect
    </button>
  );

  const renderRow = (r: Row) => (
    <div className="lm-row" key={r.key}>
      <span className={`lm-tile ${r.projector ? 'projector' : ''}`.trim()} aria-hidden="true">
        {r.projector ? <Projector size={17} /> : <Router size={17} />}
      </span>
      <div className="lm-row-text">
        <div className="lm-row-name">{r.name}</div>
        <div className="lm-row-sub">{r.sub}</div>
      </div>
      <span className="lm-row-meta">
        {r.secured && <Lock size={13} aria-label="Secured" />}
        {r.signal !== undefined && <SignalIcon signal={r.signal} />}
      </span>
      {connectButton(() => connectRow(r), `Connect to ${r.name}`)}
    </div>
  );

  return (
    <div className="lm-app">
      <header className="lm-toolbar" onMouseDown={dragWindow}>
        <span className="lm-toolbar-title">LibreMP</span>
        <label className="lm-search">
          <Search size={14} aria-hidden="true" />
          <input
            type="search"
            placeholder="Search"
            aria-label="Search projectors and networks"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </label>
        <button type="button" className="lm-icon-btn" onClick={scan} disabled={busy || scanning} title="Refresh" aria-label="Refresh">
          <RefreshCw size={16} className={scanning ? 'lm-spin' : ''} />
        </button>
        <button type="button" className="lm-icon-btn" onClick={() => setSheet('settings')} title="Settings" aria-label="Settings">
          <Settings size={16} />
        </button>
        <button type="button" className="lm-icon-btn" onClick={() => setSheet('help')} title="Help" aria-label="Help">
          <CircleHelp size={16} />
        </button>
        {ownChrome && (
          <button
            type="button"
            className="lm-icon-btn lm-close"
            onClick={() => getCurrentWindow().close()}
            title="Close"
            aria-label="Close LibreMP"
          >
            <X size={16} />
          </button>
        )}
      </header>

      <main className="lm-main">
        <div className="lm-column">
          <section className="lm-status" aria-live="polite">
            {status.kind === 'casting' ? (
              <>
                <div className="lm-status-head">
                  <span className="lm-live-dot" aria-hidden="true" />
                  <h1 className="lm-status-title">Casting to {status.target.name}</h1>
                </div>
                <p className="lm-status-text">Your screen is showing on the projector.</p>
                <div className="lm-actions">
                  <button type="button" className="lm-btn large destructive" onClick={stop}>
                    Stop Casting
                  </button>
                </div>
              </>
            ) : status.kind === 'working' ? (
              <>
                <div className="lm-status-head">
                  <LoaderCircle size={20} className="lm-spin" aria-hidden="true" />
                  <h1 className="lm-status-title">Connecting to {status.target.name}</h1>
                </div>
                <p className="lm-status-text">{status.detail}</p>
                <div className="lm-actions">
                  <button type="button" className="lm-btn large" onClick={stop} disabled={!status.cancellable}>
                    Cancel
                  </button>
                </div>
              </>
            ) : (
              <>
                <h1 className="lm-status-title">Connect to a Projector</h1>
                <p className="lm-status-text">
                  Scan the QR code on the projector’s network screen. LibreMP joins the projector and starts casting.
                </p>
                {status.kind === 'error' && (
                  <div className="lm-callout" role="alert">
                    <CircleAlert size={16} />
                    <p>{status.message}</p>
                  </div>
                )}
                <div className="lm-actions">
                  {status.kind === 'error' && status.rejected && status.target ? (
                    <button type="button" className="lm-btn large primary" onClick={() => setSheet('keyword')}>
                      Enter Keyword…
                    </button>
                  ) : (
                    <button type="button" className="lm-btn large primary" onClick={() => setSheet('scan')}>
                      <Camera size={16} /> Scan QR Code
                    </button>
                  )}
                  {status.kind === 'error' && status.target && !status.rejected && (
                    <button type="button" className="lm-btn large" onClick={() => connect(status.target!)}>
                      Try Again
                    </button>
                  )}
                  <button type="button" className="lm-btn large" onClick={() => uploadRef.current?.click()}>
                    Choose Photo…
                  </button>
                  <button type="button" className="lm-btn large" onClick={() => setSheet('manual')}>
                    Enter Manually…
                  </button>
                </div>
              </>
            )}
          </section>

          {savedRows.length > 0 && (
            <section aria-labelledby="lm-saved">
              <h2 className="lm-group-head" id="lm-saved">Saved</h2>
              <div className="lm-group">
                {savedRows.map((p) => {
                  const name = p.name || projName(p.ssid);
                  return (
                    <div className="lm-row" key={p.ssid || p.name}>
                      <span className="lm-tile projector" aria-hidden="true">
                        <Projector size={17} />
                      </span>
                      <div className="lm-row-text">
                        <div className="lm-row-name">{name}</div>
                        <div className="lm-row-sub">{p.ip || p.ssid}</div>
                      </div>
                      {connectButton(() => connectSaved(p), `Connect to ${name}`)}
                      <button
                        type="button"
                        className="lm-icon-btn"
                        disabled={busy}
                        onClick={() => forget(p)}
                        title="Forget"
                        aria-label={`Forget ${name}`}
                      >
                        <Trash2 size={15} />
                      </button>
                    </div>
                  );
                })}
              </div>
            </section>
          )}

          <section aria-labelledby="lm-projectors">
            <h2 className="lm-group-head" id="lm-projectors">
              Projectors Nearby
              {scanning && <LoaderCircle size={13} className="lm-spin" aria-label="Searching" />}
            </h2>
            <div className="lm-group">
              {projectorRows.length > 0 ? (
                projectorRows.map(renderRow)
              ) : (
                <p className="lm-empty">
                  {scanning ? 'Searching…' : 'No projectors found. Scan the projector’s QR code instead.'}
                </p>
              )}
            </div>
          </section>

          {otherRows.length > 0 && (
            <section>
              <button
                type="button"
                className="lm-disclosure"
                aria-expanded={showOthers}
                aria-controls="lm-others"
                onClick={() => setShowOthers(!showOthers)}
              >
                <ChevronRight size={14} aria-hidden="true" />
                Other Networks
                <span className="lm-count">{otherRows.length}</span>
              </button>
              {showOthers && (
                <div className="lm-group" id="lm-others">
                  {otherRows.map(renderRow)}
                </div>
              )}
            </section>
          )}
        </div>
      </main>

      <input
        ref={uploadRef}
        type="file"
        accept="image/*"
        hidden
        onChange={(e) => {
          const f = e.target.files?.[0];
          if (f) upload(f);
          e.currentTarget.value = '';
        }}
      />

      <SettingsModal isOpen={sheet === 'settings'} onClose={() => setSheet(null)} settings={settings} onApply={setSettings} />
      <HelpModal isOpen={sheet === 'help'} onClose={() => setSheet(null)} />
      <LiveScanModal isOpen={sheet === 'scan'} onClose={() => setSheet(null)} onDecoded={fromQr} />
      <ManualConnectModal
        isOpen={sheet === 'manual'}
        onClose={() => setSheet(null)}
        onConnect={(ssid, password) => {
          setSheet(null);
          connect({ name: projName(ssid), ssid, password, ip: '', joinWifi: true });
        }}
      />
      <KeywordModal
        isOpen={sheet === 'keyword'}
        projectorName={status.kind === 'error' && status.target ? status.target.name : 'The projector'}
        onClose={() => setSheet(null)}
        onSubmit={(keyword) => {
          setSheet(null);
          if (status.kind === 'error' && status.target) connect({ ...status.target, keyword });
        }}
      />
      <PasswordModal
        isOpen={!!passwordFor}
        networkName={passwordFor?.name ?? ''}
        isProjector={!!passwordFor && passwordFor.name !== passwordFor.ssid}
        isLoading={status.kind === 'working' && status.target === passwordFor}
        error={passwordError}
        onCancel={() => setPasswordFor(null)}
        onSubmit={async (password) => {
          if (!passwordFor) return;
          const t = { ...passwordFor, password };
          setPasswordFor(t);
          setPasswordError(null);
          const ok = await connect(t);
          if (ok) setPasswordFor(null);
          else setPasswordError('Could not join. Check the password and try again.');
        }}
      />

      <Toast toast={toast} onDismiss={dismissToast} />
    </div>
  );
}

export default App;
