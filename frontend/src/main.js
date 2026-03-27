/* ═══════════════════════════════════════════════════════════
   Libre MP — Main Application Logic
   ═══════════════════════════════════════════════════════════ */
import { injectIcon, icon } from './icons.js';

// ── Tauri API (available when running inside Tauri) ──
const { invoke } = window.__TAURI__?.core ?? { invoke: mockInvoke };

// ═══════════════════════════════════════════════════════════
//  STATE
// ═══════════════════════════════════════════════════════════
let allProjectors = [];
let recentConnections = [];
let currentStatus = 'disconnected'; // disconnected | searching | connected
let connectedSSID = null;
let scanInterval = null;

// ═══════════════════════════════════════════════════════════
//  INIT
// ═══════════════════════════════════════════════════════════
document.addEventListener('DOMContentLoaded', () => {
  injectIcons();
  bindEvents();
  loadRecentConnections();
  startScan();
});

function injectIcons() {
  // Status bar
  document.getElementById('btn-settings-toggle').innerHTML = icon('settings');

  // Search box
  document.getElementById('icon-search').innerHTML = icon('search', 16);
  document.getElementById('btn-filter').innerHTML = icon('filter', 16);

  // Section titles
  document.getElementById('icon-recent').innerHTML = icon('clock', 16);
  document.getElementById('icon-projector-title').innerHTML = icon('projector', 16);
  document.getElementById('icon-refresh').innerHTML = icon('refresh', 16);

  // Settings panel
  document.getElementById('icon-settings-title').innerHTML = icon('settings', 16);
  document.getElementById('btn-settings-close').innerHTML = icon('close');
  document.getElementById('icon-display').innerHTML = icon('display', 16);
  document.getElementById('icon-brightness').innerHTML = icon('brightness', 16);
  document.getElementById('icon-resolution').innerHTML = icon('resolution', 16);
  document.getElementById('icon-bandwidth').innerHTML = icon('bandwidth', 16);
  document.getElementById('icon-audio').innerHTML = icon('audio', 16);
  document.getElementById('icon-reconnect').innerHTML = icon('reconnect', 16);

  // Help FAB
  document.getElementById('btn-help').innerHTML = icon('help', 24);

  // Help close
  document.getElementById('btn-help-close').innerHTML = icon('close');
}

// ═══════════════════════════════════════════════════════════
//  EVENT BINDINGS
// ═══════════════════════════════════════════════════════════
function bindEvents() {
  // Search
  document.getElementById('search-input').addEventListener('input', (e) => {
    renderProjectorList(filterProjectors(e.target.value));
  });

  // Filter toggle
  document.getElementById('btn-filter').addEventListener('click', () => {
    document.getElementById('filter-dropdown').classList.toggle('hidden');
  });

  // Filter checkboxes
  document.getElementById('filter-epson').addEventListener('change', applyFilters);
  document.getElementById('filter-strong').addEventListener('change', applyFilters);

  // Refresh
  document.getElementById('btn-refresh').addEventListener('click', () => {
    startScan();
  });

  // Settings panel open/close
  document.getElementById('btn-settings-toggle').addEventListener('click', openSettings);
  document.getElementById('btn-settings-close').addEventListener('click', closeSettings);
  document.getElementById('settings-overlay').addEventListener('click', closeSettings);

  // Settings controls
  document.getElementById('brightness-slider').addEventListener('input', (e) => {
    document.getElementById('brightness-value').textContent = e.target.value + '%';
  });

  document.getElementById('btn-apply-settings').addEventListener('click', applySettings);
  document.getElementById('btn-reset-settings').addEventListener('click', resetSettings);

  // Help
  document.getElementById('btn-help').addEventListener('click', openHelp);
  document.getElementById('btn-help-close').addEventListener('click', closeHelp);
  document.getElementById('help-overlay').addEventListener('click', (e) => {
    if (e.target === e.currentTarget) closeHelp();
  });

  // Keyboard shortcuts
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      closeSettings();
      closeHelp();
      document.getElementById('filter-dropdown').classList.add('hidden');
    }
  });
}

// ═══════════════════════════════════════════════════════════
//  WIFI SCAN & CONNECTION
// ═══════════════════════════════════════════════════════════
async function startScan() {
  setStatus('searching');
  showScanSpinner(true);

  try {
    const results = await invoke('scan_wifi');
    allProjectors = typeof results === 'string' ? JSON.parse(results) : results;
    showScanSpinner(false);

    if (allProjectors.length > 0) {
      renderProjectorList(filterProjectors(document.getElementById('search-input').value));
      // Check auto-reconnect
      const autoReconnect = document.getElementById('autoreconnect-toggle').checked;
      if (autoReconnect && recentConnections.length > 0) {
        const lastSSID = recentConnections[0].ssid;
        const match = allProjectors.find(p => p.ssid === lastSSID);
        if (match && currentStatus !== 'connected') {
          connectToProjector(match.ssid, match.bssid);
        }
      }
      if (currentStatus !== 'connected') {
        setStatus('disconnected');
      }
    } else {
      setStatus('disconnected');
    }
  } catch (err) {
    console.error('Scan failed:', err);
    showScanSpinner(false);
    setStatus('disconnected');
  }

  // Auto-rescan every 10 seconds
  clearInterval(scanInterval);
  scanInterval = setInterval(() => {
    if (currentStatus !== 'connected') startScan();
  }, 10000);
}

async function connectToProjector(ssid, bssid) {
  setStatus('searching');
  const card = document.querySelector(`[data-bssid="${bssid}"]`);
  if (card) {
    const btn = card.querySelector('.btn-connect');
    btn.classList.add('btn-connect--connecting');
    btn.innerHTML = `<span class="spinner spinner--sm"></span> Connecting…`;
  }

  try {
    const result = await invoke('connect_projector', { ssid, bssid });
    if (result === true || result === 'true' || result?.success) {
      setStatus('connected');
      connectedSSID = ssid;
      saveRecentConnection(ssid, bssid);
      renderProjectorList(filterProjectors(document.getElementById('search-input').value));
    } else {
      setStatus('disconnected');
      if (card) {
        const btn = card.querySelector('.btn-connect');
        btn.classList.remove('btn-connect--connecting');
        btn.textContent = 'Connect';
      }
    }
  } catch (err) {
    console.error('Connect failed:', err);
    setStatus('disconnected');
    if (card) {
      const btn = card.querySelector('.btn-connect');
      btn.classList.remove('btn-connect--connecting');
      btn.textContent = 'Retry';
    }
  }
}

// ═══════════════════════════════════════════════════════════
//  STATUS BANNER
// ═══════════════════════════════════════════════════════════
function setStatus(status) {
  currentStatus = status;
  const banner = document.getElementById('status-banner');
  banner.dataset.status = status;
  const labels = { disconnected: 'Disconnected', searching: 'Searching…', connected: 'Connected' };
  banner.querySelector('.status-text').textContent = labels[status] || status;
}

// ═══════════════════════════════════════════════════════════
//  FILTERING
// ═══════════════════════════════════════════════════════════
function filterProjectors(query = '') {
  let filtered = [...allProjectors];

  // Text search
  if (query.trim()) {
    const q = query.toLowerCase();
    filtered = filtered.filter(p =>
      (p.ssid || '').toLowerCase().includes(q) ||
      (p.name || '').toLowerCase().includes(q) ||
      (p.bssid || '').toLowerCase().includes(q)
    );
  }

  // Epson-only filter
  if (document.getElementById('filter-epson')?.checked) {
    // Keep all for now — the backend already filters for Epson SSIDs
  }

  // Strong signal filter
  if (document.getElementById('filter-strong')?.checked) {
    filtered = filtered.filter(p => (p.signal || 0) >= 60);
  }

  return filtered;
}

function applyFilters() {
  renderProjectorList(filterProjectors(document.getElementById('search-input').value));
}

// ═══════════════════════════════════════════════════════════
//  RENDERING
// ═══════════════════════════════════════════════════════════
function renderProjectorList(projectors) {
  const container = document.getElementById('projector-list');
  const empty = document.getElementById('projectors-empty');

  // Remove old cards
  container.querySelectorAll('.projector-card').forEach(c => c.remove());

  if (projectors.length === 0) {
    empty.classList.remove('hidden');
    return;
  }

  empty.classList.add('hidden');

  projectors.forEach((p, i) => {
    const strength = signalToStrength(p.signal || 0);
    const isConnected = connectedSSID === p.ssid && currentStatus === 'connected';

    const card = document.createElement('div');
    card.className = 'projector-card card';
    card.dataset.bssid = p.bssid || '';
    card.style.animationDelay = `${i * 60}ms`;

    card.innerHTML = `
      <div class="projector-card__icon">${icon('projector', 24)}</div>
      <div class="projector-card__info">
        <div class="projector-card__name">${escapeHtml(p.ssid || p.name || 'Unknown')}</div>
        <div class="projector-card__ssid">BSSID: ${escapeHtml(p.bssid || '—')}</div>
        ${p.security ? `<div class="projector-card__security">${escapeHtml(p.security)}</div>` : ''}
      </div>
      ${renderSignalBars(strength)}
      <button class="btn-connect ${isConnected ? 'btn-connect--connected' : ''}"
              ${isConnected ? 'disabled' : ''}
              data-ssid="${escapeAttr(p.ssid)}"
              data-bssid="${escapeAttr(p.bssid)}">
        ${isConnected ? '✓ Connected' : 'Connect'}
      </button>
    `;

    if (!isConnected) {
      card.querySelector('.btn-connect').addEventListener('click', (e) => {
        const ssid = e.currentTarget.dataset.ssid;
        const bssid = e.currentTarget.dataset.bssid;
        connectToProjector(ssid, bssid);
      });
    }

    container.appendChild(card);
  });
}

function renderRecentList() {
  const container = document.getElementById('recent-list');
  const empty = document.getElementById('recent-empty');

  container.querySelectorAll('.recent-card').forEach(c => c.remove());

  if (recentConnections.length === 0) {
    empty.classList.remove('hidden');
    return;
  }

  empty.classList.add('hidden');

  recentConnections.slice(0, 5).forEach((rc) => {
    const card = document.createElement('div');
    card.className = 'recent-card card';

    card.innerHTML = `
      <div class="recent-card__icon">${icon('projector', 20)}</div>
      <div class="recent-card__info">
        <div class="recent-card__name">${escapeHtml(rc.ssid)}</div>
        <div class="recent-card__ssid">${escapeHtml(rc.bssid || '')}</div>
      </div>
      <button class="btn-quick-connect"
              data-ssid="${escapeAttr(rc.ssid)}"
              data-bssid="${escapeAttr(rc.bssid)}">
        Quick connect
      </button>
    `;

    card.querySelector('.btn-quick-connect').addEventListener('click', (e) => {
      e.stopPropagation();
      const ssid = e.currentTarget.dataset.ssid;
      const bssid = e.currentTarget.dataset.bssid;
      connectToProjector(ssid, bssid);
    });

    container.appendChild(card);
  });
}

// ═══════════════════════════════════════════════════════════
//  SIGNAL STRENGTH
// ═══════════════════════════════════════════════════════════
function signalToStrength(signal) {
  if (signal >= 80) return 5;
  if (signal >= 60) return 4;
  if (signal >= 40) return 3;
  if (signal >= 20) return 2;
  if (signal > 0)   return 1;
  return 0;
}

function renderSignalBars(strength) {
  return `
    <div class="signal-bars" data-strength="${strength}" title="Signal: ${strength}/5">
      <div class="signal-bar"></div>
      <div class="signal-bar"></div>
      <div class="signal-bar"></div>
      <div class="signal-bar"></div>
      <div class="signal-bar"></div>
    </div>
  `;
}

// ═══════════════════════════════════════════════════════════
//  RECENT CONNECTIONS PERSISTENCE
// ═══════════════════════════════════════════════════════════
async function loadRecentConnections() {
  try {
    const data = await invoke('get_recent');
    recentConnections = typeof data === 'string' ? JSON.parse(data) : (data || []);
  } catch {
    // Fallback to localStorage
    try {
      recentConnections = JSON.parse(localStorage.getItem('recentConnections') || '[]');
    } catch { recentConnections = []; }
  }
  renderRecentList();
}

async function saveRecentConnection(ssid, bssid) {
  // Remove duplicates, add to front
  recentConnections = recentConnections.filter(r => r.ssid !== ssid);
  recentConnections.unshift({ ssid, bssid, timestamp: Date.now() });
  recentConnections = recentConnections.slice(0, 5);

  try {
    await invoke('save_recent', { entries: JSON.stringify(recentConnections) });
  } catch {
    localStorage.setItem('recentConnections', JSON.stringify(recentConnections));
  }
  renderRecentList();
}

// ═══════════════════════════════════════════════════════════
//  SETTINGS
// ═══════════════════════════════════════════════════════════
function openSettings() {
  document.getElementById('settings-panel').classList.remove('hidden');
  document.getElementById('settings-overlay').classList.remove('hidden');
}

function closeSettings() {
  document.getElementById('settings-panel').classList.add('hidden');
  document.getElementById('settings-overlay').classList.add('hidden');
}

async function applySettings() {
  const settings = {
    displayMode: document.querySelector('input[name="display-mode"]:checked')?.value || 'operations',
    brightness: parseInt(document.getElementById('brightness-slider').value),
    resolution: document.getElementById('resolution-select').value,
    bandwidth: document.getElementById('bandwidth-select').value,
    audio: document.getElementById('audio-toggle').checked,
    autoReconnect: document.getElementById('autoreconnect-toggle').checked,
  };

  try {
    await invoke('save_settings', { settings: JSON.stringify(settings) });
  } catch {
    localStorage.setItem('settings', JSON.stringify(settings));
  }

  closeSettings();
}

function resetSettings() {
  document.querySelector('input[name="display-mode"][value="operations"]').checked = true;
  document.getElementById('brightness-slider').value = 75;
  document.getElementById('brightness-value').textContent = '75%';
  document.getElementById('resolution-select').value = '1600x900';
  document.getElementById('bandwidth-select').value = '15';
  document.getElementById('audio-toggle').checked = false;
  document.getElementById('autoreconnect-toggle').checked = true;
}

// ═══════════════════════════════════════════════════════════
//  HELP
// ═══════════════════════════════════════════════════════════
function openHelp() {
  document.getElementById('help-overlay').classList.remove('hidden');
}

function closeHelp() {
  document.getElementById('help-overlay').classList.add('hidden');
}

// ═══════════════════════════════════════════════════════════
//  SCANNER SPINNER
// ═══════════════════════════════════════════════════════════
function showScanSpinner(show) {
  const spinner = document.getElementById('scan-spinner');
  const empty = document.getElementById('projectors-empty');
  if (show) {
    empty.classList.remove('hidden');
    spinner.style.display = '';
    empty.querySelector('.empty-state__text').textContent = 'Scanning for projectors…';
  } else {
    spinner.style.display = 'none';
  }
}

// ═══════════════════════════════════════════════════════════
//  UTILITIES
// ═══════════════════════════════════════════════════════════
function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

function escapeAttr(str) {
  return (str || '').replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}

// ═══════════════════════════════════════════════════════════
//  MOCK INVOKE (when running outside Tauri, e.g., in browser)
// ═══════════════════════════════════════════════════════════
async function mockInvoke(cmd, args = {}) {
  console.log(`[mock] invoke('${cmd}', ${JSON.stringify(args)})`);

  // Simulated delay
  await new Promise(r => setTimeout(r, 800));

  switch (cmd) {
    case 'scan_wifi':
      return [
        { ssid: 'RESEARCHLAB', bssid: 'A0:B1:C2:D3:E4:F5', signal: 85, security: 'WPA2' },
        { ssid: 'EPSON-Projector-Lab2', bssid: 'F6:E5:D4:C3:B2:A1', signal: 72, security: 'WPA2' },
        { ssid: 'CONFERENCE-ROOM-A', bssid: '11:22:33:44:55:66', signal: 55, security: 'WPA2' },
        { ssid: 'EPSON-EB-W06', bssid: 'AA:BB:CC:DD:EE:FF', signal: 30, security: 'Open' },
        { ssid: 'RESEARCH-2', bssid: '12:34:56:78:9A:BC', signal: 92, security: 'WPA3' },
      ];

    case 'connect_projector':
      return true;

    case 'get_recent':
      return JSON.parse(localStorage.getItem('recentConnections') || '[]');

    case 'save_recent':
      localStorage.setItem('recentConnections', args.entries);
      return true;

    case 'get_settings':
      return JSON.parse(localStorage.getItem('settings') || '{}');

    case 'save_settings':
      localStorage.setItem('settings', args.settings);
      return true;

    default:
      return null;
  }
}
