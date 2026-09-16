use serde::{Deserialize, Serialize};
use std::process::Command;
use regex::Regex;
use lazy_static::lazy_static;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Requests sent to the dedicated camera thread. The thread owns the (non-Send)
/// `nokhwa::Camera`, so the handle never has to live in Tauri's shared state.
enum CamCmd {
    Preview(tokio::sync::oneshot::Sender<Result<String, String>>),
    Capture(tokio::sync::oneshot::Sender<Result<String, String>>),
    Scan(tokio::sync::oneshot::Sender<Result<QrProjector, String>>),
    Stop,
}

pub struct AppState {
    pub child_process: Arc<Mutex<Option<std::process::Child>>>,
    /// The Wi-Fi we were on before joining a projector, restored when casting
    /// stops. Linux: a NetworkManager UUID. macOS: an SSID.
    prev_wifi: Arc<Mutex<Option<String>>>,
    /// Sender to the camera worker thread. `None` until a scan opens the camera;
    /// reset to `None` by `camera_stop`.
    cam_tx: Arc<Mutex<Option<std::sync::mpsc::Sender<CamCmd>>>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectorInfo {
    pub name: String,
    pub ip: String,
    pub model: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub bssid: String,
    pub signal: u8,
    pub security: String,
    pub is_projector: bool,
}

lazy_static! {
    static ref SSID_RE: Regex = Regex::new(r"(?m)^[^B]*SSID\s+\d+\s+:\s+(.*)").unwrap();
    static ref BSSID_RE: Regex = Regex::new(r"BSSID\s+\d+\s+:\s+([0-9a-fA-F:]{17})").unwrap();
    static ref SIGNAL_RE: Regex = Regex::new(r"Signal\s+:\s+(\d+)%").unwrap();
    static ref AUTH_RE: Regex = Regex::new(r"Authentication\s+:\s+([^\r\n]+)").unwrap();
}

/// `Command::new` that does not flash a console window on Windows. The GUI runs
/// netsh every scan (every 12 s), so without this a black window blinks each time.
fn hidden_cmd<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// The Wi-Fi connection we are on right now: a NetworkManager UUID on Linux, the
/// SSID on macOS. Wired and loopback connections are ignored on purpose — only a
/// Wi-Fi connection can be replaced by joining a projector.
fn current_wifi_id() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let device = macos_wifi_device()?;
        let out = hidden_cmd("networksetup").args(["-getairportnetwork", &device]).output().ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        return text
            .split_once("Current Wi-Fi Network: ")
            .map(|(_, n)| n.trim().to_string())
            .filter(|n| !n.is_empty());
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let out = hidden_cmd("nmcli")
            .args(["-t", "-f", "UUID,TYPE", "connection", "show", "--active"])
            .output()
            .ok()?;
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            if let Some((uuid, kind)) = line.rsplit_once(':') {
                if kind.contains("wireless") {
                    return Some(uuid.to_string());
                }
            }
        }
        None
    }
    #[cfg(target_os = "windows")]
    {
        None // Windows restore is not wired up yet.
    }
}

/// Rejoins the network saved by [`current_wifi_id`]. Best-effort: if it fails the
/// user is simply left on the projector's network, as before.
fn restore_wifi(id: &str) {
    #[cfg(target_os = "macos")]
    if let Some(device) = macos_wifi_device() {
        let _ = hidden_cmd("networksetup").args(["-setairportnetwork", &device, id]).output();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = hidden_cmd("nmcli").args(["connection", "up", "uuid", id]).output();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = id;
    }
}

/// Escapes text for an XML element (Wi-Fi names and passphrases may contain `&`, `<`, ...).
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

/// Heuristic for flagging an SSID as a likely projector (Epson direct-mode, etc.).
fn is_projector_ssid(ssid: &str) -> bool {
    let l = ssid.to_lowercase();
    l.contains("epson")
        || l.contains("projector")
        || l.contains("direct-")
        || l.contains("display")
        || l.contains("cast")
}

/// Detects the active Wi-Fi hardware device name on macOS (e.g. "en0" or "en1").
/// The Wi-Fi port is NOT always en0, so we look it up instead of assuming.
#[cfg(target_os = "macos")]
fn macos_wifi_device() -> Option<String> {
    let out = hidden_cmd("networksetup")
        .arg("-listallhardwareports")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut is_wifi = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(port) = line.strip_prefix("Hardware Port:") {
            is_wifi = port.contains("Wi-Fi") || port.contains("AirPort");
        } else if is_wifi {
            if let Some(dev) = line.strip_prefix("Device:") {
                return Some(dev.trim().to_string());
            }
        }
    }
    None
}

/// Converts a macOS "spairport_signal_noise" field (e.g. "-55 dBm / -90 dBm")
/// into a rough 0-100 signal percentage.
#[cfg(target_os = "macos")]
fn macos_signal_to_percent(field: &str) -> u8 {
    let rssi: i32 = field
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(-100);
    // -50 dBm ≈ excellent, -100 dBm ≈ none.
    (2 * (rssi + 100)).clamp(0, 100) as u8
}

/// Normalizes macOS "spairport_security_mode" strings into the same vocabulary
/// the frontend expects ("Open" means no password prompt).
#[cfg(target_os = "macos")]
fn macos_clean_security(mode: &str) -> String {
    let m = mode.to_lowercase();
    if m.contains("none") || m.is_empty() {
        "Open".to_string()
    } else if m.contains("wep") {
        "WEP".to_string()
    } else {
        "WPA2".to_string()
    }
}

/// Scans for available Wi-Fi networks using OS-native tools (nmcli/netsh/system_profiler).
#[tauri::command]
#[allow(unreachable_code)]
async fn scan_wifi_networks() -> Result<Vec<WifiNetwork>, String> {
    // ── macOS ── `nmcli` does not exist; use system_profiler (airport CLI was
    // removed in macOS 14). This requires Location Services to see SSIDs.
    #[cfg(target_os = "macos")]
    {
        let output = hidden_cmd("system_profiler")
            .args(["SPAirPortDataType", "-json"])
            .output()
            .map_err(|e| format!("Failed to execute system_profiler: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut networks: Vec<WifiNetwork> = Vec::new();

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(ifaces) = json
                .get("SPAirPortDataType")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|v| v.get("spairport_airport_interfaces"))
                .and_then(|v| v.as_array())
            {
                for iface in ifaces {
                    // Visible networks plus the one we are currently joined to.
                    let mut buckets: Vec<&serde_json::Value> = Vec::new();
                    if let Some(arr) = iface
                        .get("spairport_airport_other_local_wireless_networks")
                        .and_then(|v| v.as_array())
                    {
                        buckets.extend(arr.iter());
                    }
                    if let Some(cur) = iface.get("spairport_current_network_information") {
                        buckets.push(cur);
                    }

                    for net in buckets {
                        let ssid = match net.get("_name").and_then(|v| v.as_str()) {
                            Some(s) if !s.is_empty() => s.to_string(),
                            _ => continue,
                        };
                        let security = net
                            .get("spairport_security_mode")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let signal = net
                            .get("spairport_signal_noise")
                            .and_then(|v| v.as_str())
                            .map(macos_signal_to_percent)
                            .unwrap_or(0);

                        networks.push(WifiNetwork {
                            is_projector: is_projector_ssid(&ssid),
                            ssid,
                            bssid: String::new(),
                            signal,
                            security: macos_clean_security(security),
                        });
                    }
                }
            }
        }

        let mut unique_nets: std::collections::HashMap<String, WifiNetwork> =
            std::collections::HashMap::new();
        for n in networks {
            let entry = unique_nets.entry(n.ssid.clone()).or_insert_with(|| n.clone());
            if n.signal > entry.signal {
                *entry = n;
            }
        }
        let mut result: Vec<WifiNetwork> = unique_nets.into_values().collect();
        result.sort_by(|a, b| b.signal.cmp(&a.signal));
        return Ok(result);
    }

    // ── Linux (NetworkManager) ──
    if !cfg!(target_os = "windows") && !cfg!(target_os = "macos") {
        let output = hidden_cmd("nmcli")
            .args(["-t", "-f", "SSID,BSSID,SECURITY,SIGNAL", "dev", "wifi"])
            .output()
            .map_err(|e| format!("Failed to execute nmcli: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut networks = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }
            
            let unescaped = line.replace("\\:", "%%COLON%%");
            let parts: Vec<&str> = unescaped.split(':').collect();
            if parts.len() >= 4 {
                let ssid = parts[0].replace("%%COLON%%", ":");
                let bssid = parts[1].replace("%%COLON%%", ":");
                let security = parts[2].to_string();
                let signal = parts[3].parse::<u8>().unwrap_or(0);
                
                if !ssid.is_empty() && ssid != "--" {
                    let is_projector = is_projector_ssid(&ssid);
                    networks.push(WifiNetwork {
                        ssid,
                        bssid,
                        signal,
                        security,
                        is_projector,
                    });
                }
            }
        }
        
        let mut unique_nets: std::collections::HashMap<String, WifiNetwork> = std::collections::HashMap::new();
        for n in networks {
            let entry = unique_nets.entry(n.ssid.clone()).or_insert_with(|| n.clone());
            if n.signal > entry.signal {
                *entry = n;
            }
        }

        let mut result: Vec<WifiNetwork> = unique_nets.into_values().collect();
        result.sort_by(|a, b| b.signal.cmp(&a.signal));
        return Ok(result);
    }

    let output = hidden_cmd("netsh")
        .args(["wlan", "show", "networks", "mode=bssid"])
        .output()
        .map_err(|e| format!("Failed to execute netsh: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut networks = Vec::new();
    
    // Line-by-line state machine parser
    let mut current_ssid = String::new();
    let mut current_security = String::from("Open");
    let mut current_bssid = String::new();
    let mut current_signal = 0;

    let ssid_line_re = Regex::new(r"^SSID\s+\d+\s*:\s*(.*)").unwrap();

    for line in stdout.lines() {
        let trimmed = line.trim();

        // New SSID block starts
        if let Some(caps) = ssid_line_re.captures(trimmed) {
            if !current_ssid.is_empty() {
                let bssid = if current_bssid.is_empty() {
                    format!("unknown-{}", networks.len())
                } else {
                    current_bssid.clone()
                };
                networks.push(WifiNetwork {
                    ssid: current_ssid.clone(),
                    bssid,
                    signal: current_signal.max(1), // Default minimal signal if visible but no signal reported
                    security: current_security.clone(),
                    is_projector: is_projector_ssid(&current_ssid),
                });
            }

            current_ssid = caps[1].trim().to_string();
            current_security = String::from("Open");
            current_bssid = String::new();
            current_signal = 0;
            continue;
        }

        // Skip if no SSID context yet
        if current_ssid.is_empty() { continue; }

        // Auth line
        if let Some(caps) = AUTH_RE.captures(trimmed) {
            current_security = caps[1].trim().to_string();
            continue;
        }

        // BSSID line
        if let Some(caps) = BSSID_RE.captures(trimmed) {
            if !current_bssid.is_empty() {
                networks.push(WifiNetwork {
                    ssid: current_ssid.clone(),
                    bssid: current_bssid.clone(),
                    signal: current_signal.max(1),
                    security: current_security.clone(),
                    is_projector: is_projector_ssid(&current_ssid),
                });
                current_signal = 0;
            }
            current_bssid = caps[1].to_string();
            continue;
        }

        // Signal line
        if let Some(caps) = SIGNAL_RE.captures(trimmed) {
            if let Ok(sig) = caps[1].parse::<u8>() {
                current_signal = sig;
            }
            continue;
        }
    }

    if !current_ssid.is_empty() {
        let bssid = if current_bssid.is_empty() {
            format!("unknown-{}", networks.len())
        } else {
            current_bssid.clone()
        };
        networks.push(WifiNetwork {
            ssid: current_ssid.clone(),
            bssid,
            signal: current_signal.max(1),
            security: current_security.clone(),
            is_projector: {
                let l = current_ssid.to_lowercase();
                l.contains("epson") || l.contains("projector") || l.contains("direct-") || l.contains("display") || l.contains("cast")
            },
        });
    }
    
    // Deduplicate by SSID, keeping strongest signal
    let mut unique_nets: std::collections::HashMap<String, WifiNetwork> = std::collections::HashMap::new();
    for n in networks {
        let entry = unique_nets.entry(n.ssid.clone()).or_insert_with(|| n.clone());
        if n.signal > entry.signal {
            *entry = n;
        }
    }

    let mut result: Vec<WifiNetwork> = unique_nets.into_values().collect();
    result.sort_by(|a, b| b.signal.cmp(&a.signal));
    Ok(result)

}

/// Credentials decoded from an Epson Quick Connect QR code.
#[derive(Debug, Serialize, Deserialize)]
pub struct QrProjector {
    pub ssid: String,
    pub password: String,
    pub ip: String,
}

/// Decodes an Epson projector QR code from an uploaded image's raw bytes.
/// Returns the SSID, Wi-Fi passphrase, and Direct-mode IP so the app can
/// auto-connect without the user typing anything.
#[tauri::command]
async fn decode_projector_qr(image_bytes: Vec<u8>) -> Result<QrProjector, String> {
    let qr = libremp_core::qr::parse_from_image_bytes(&image_bytes).ok_or_else(|| {
        "No Epson projector QR code found. Make sure the QR is clear and fills the frame."
            .to_string()
    })?;
    Ok(QrProjector {
        ssid: qr.ssid().unwrap_or_default().to_string(),
        password: qr.wifi_password().unwrap_or_default().to_string(),
        ip: qr.ip.to_string(),
    })
}

// ── Live camera: preview → capture → scan ──────────────────────────────────
// Native camera (V4L2/AVFoundation/MediaFoundation), NOT the webview's
// getUserMedia (which segfaults WebKitGTK). Frames are JPEG-encoded and streamed
// to the UI as data: URLs for a real preview.

/// Ensure the camera is open in `guard` and grab one RGB frame (w, h, rgb).
fn grab_rgb(guard: &mut Option<nokhwa::Camera>) -> Result<(u32, u32, Vec<u8>), String> {
    use nokhwa::pixel_format::RgbFormat;
    use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
    use nokhwa::Camera;

    if guard.is_none() {
        let format =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution);
        let mut camera = Camera::new(CameraIndex::Index(0), format)
            .map_err(|e| format!("Could not open the camera: {e}"))?;
        camera
            .open_stream()
            .map_err(|e| format!("Could not start the camera: {e}"))?;
        *guard = Some(camera);
    }
    let camera = guard.as_mut().unwrap();
    let img = camera
        .frame()
        .and_then(|f| f.decode_image::<RgbFormat>())
        .map_err(|e| e.to_string())?;
    Ok((img.width(), img.height(), img.into_raw()))
}

fn jpeg_data_url(rgb: &[u8], w: u32, h: u32, quality: i32) -> Result<String, String> {
    use base64::Engine as _;
    let jpeg = libremp_core::capture::encode_jpeg(rgb, w, h, quality)
        .ok_or_else(|| "Failed to encode frame".to_string())?;
    Ok(format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&jpeg)
    ))
}

/// The camera thread body: owns the Camera + the last captured still, and serves
/// requests until it receives `Stop` (or all senders drop). Dropping at the end
/// releases the device.
fn camera_worker(rx: std::sync::mpsc::Receiver<CamCmd>) {
    let mut cam: Option<nokhwa::Camera> = None;
    let mut captured: Option<(u32, u32, Vec<u8>)> = None;

    while let Ok(cmd) = rx.recv() {
        match cmd {
            CamCmd::Preview(reply) => {
                let r = grab_rgb(&mut cam).and_then(|(w, h, rgb)| jpeg_data_url(&rgb, w, h, 55));
                let _ = reply.send(r);
            }
            CamCmd::Capture(reply) => {
                let r = grab_rgb(&mut cam).and_then(|(w, h, rgb)| {
                    let url = jpeg_data_url(&rgb, w, h, 88)?;
                    captured = Some((w, h, rgb));
                    Ok(url)
                });
                let _ = reply.send(r);
            }
            CamCmd::Scan(reply) => {
                let r = match &captured {
                    None => Err("Take a photo first.".to_string()),
                    Some((w, h, rgb)) => libremp_core::qr::parse_from_rgb(*w, *h, rgb)
                        .map(|qr| QrProjector {
                            ssid: qr.ssid().unwrap_or_default().to_string(),
                            password: qr.wifi_password().unwrap_or_default().to_string(),
                            ip: qr.ip.to_string(),
                        })
                        .ok_or_else(|| {
                            "No projector QR found in the photo. Retake it with the QR filling more of the frame.".to_string()
                        }),
                };
                let _ = reply.send(r);
            }
            CamCmd::Stop => break,
        }
    }
}

/// Get the worker's command sender, spawning the worker thread on first use.
async fn camera_sender(state: &tauri::State<'_, AppState>) -> std::sync::mpsc::Sender<CamCmd> {
    let mut guard = state.cam_tx.lock().await;
    if let Some(tx) = guard.as_ref() {
        return tx.clone();
    }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || camera_worker(rx));
    *guard = Some(tx.clone());
    tx
}

/// Grab one live preview frame as a JPEG data: URL (opens the camera on first call).
#[tauri::command]
async fn camera_preview_frame(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let tx = camera_sender(&state).await;
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(CamCmd::Preview(reply_tx))
        .map_err(|_| "Camera stopped.".to_string())?;
    reply_rx.await.map_err(|_| "Camera stopped.".to_string())?
}

/// Freeze a full-resolution still and return it as a data: URL. Stored for `camera_scan`.
#[tauri::command]
async fn camera_capture(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let tx = camera_sender(&state).await;
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(CamCmd::Capture(reply_tx))
        .map_err(|_| "Camera stopped.".to_string())?;
    reply_rx.await.map_err(|_| "Camera stopped.".to_string())?
}

/// Decode the projector QR from the captured still.
#[tauri::command]
async fn camera_scan(state: tauri::State<'_, AppState>) -> Result<QrProjector, String> {
    let tx = camera_sender(&state).await;
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(CamCmd::Scan(reply_tx))
        .map_err(|_| "Camera stopped.".to_string())?;
    reply_rx.await.map_err(|_| "Camera stopped.".to_string())?
}

/// Release the camera (on modal close): stop the worker so it drops the device.
#[tauri::command]
async fn camera_stop(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.cam_tx.lock().await;
    if let Some(tx) = guard.take() {
        let _ = tx.send(CamCmd::Stop);
    }
    Ok(())
}

/// A projector the user has connected to before, persisted for one-click rejoin.
#[derive(Debug, Serialize, Deserialize)]
pub struct SavedProjectorDto {
    pub name: String,
    pub ssid: String,
    pub password: String,
    pub ip: String,
}

/// Lists projectors saved from previous successful connections.
#[tauri::command]
async fn list_saved_projectors() -> Result<Vec<SavedProjectorDto>, String> {
    let store = libremp_core::config::SavedProjectors::load();
    Ok(store
        .projectors
        .into_iter()
        .map(|p| SavedProjectorDto {
            name: p.name,
            ssid: p.ssid,
            password: p.psk,
            ip: p.last_ip,
        })
        .collect())
}

/// Saves (or updates) a projector so it can be rejoined later without re-entry.
#[tauri::command]
async fn save_projector(
    name: String,
    ssid: String,
    password: String,
    ip: String,
) -> Result<(), String> {
    let mut store = libremp_core::config::SavedProjectors::load();
    store.upsert(libremp_core::config::SavedProjector {
        name,
        ssid,
        psk: password.clone(),
        auth_token: password, // password IS the MAC on Epson Quick Connect
        last_ip: ip,
    });
    store.save().map_err(|e| e.to_string())
}

/// Removes a saved projector by SSID.
#[tauri::command]
async fn forget_projector(ssid: String) -> Result<(), String> {
    let mut store = libremp_core::config::SavedProjectors::load();
    store.projectors.retain(|p| p.ssid != ssid);
    store.save().map_err(|e| e.to_string())
}

/// Discovers local Epson projectors using UDP broadcast probes.
#[tauri::command]
async fn discover_projectors() -> Result<Vec<ProjectorInfo>, String> {
    use tokio::net::UdpSocket;
    use std::time::Duration;

    let socket = UdpSocket::bind("0.0.0.0:0").await.map_err(|e| e.to_string())?;
    socket.set_broadcast(true).map_err(|e| e.to_string())?;

    // Epson iProjection uses multiple discovery methods:
    // 1. ESC/VP.net broadcast on port 3629
    // 2. EEMP protocol probe on port 3620
    
    // Method 1: ESC/VP.net (port 3629). HELLO (type 0x01) is the discovery request;
    // CONNECT (0x03) is the TCP session opener, sent too for older firmware.
    let escvp_hello = b"ESC/VP.net\x10\x01\x00\x00\x00\x00";
    let escvp_msg = b"ESC/VP.net\x10\x03\x00\x00\x00\x00";
    let mut broadcasts: Vec<String> = vec!["255.255.255.255".into()];
    broadcasts.extend(local_broadcasts().iter().map(|b| b.to_string()));
    for b in &broadcasts {
        let _ = socket.send_to(escvp_hello, format!("{b}:3629")).await;
        let _ = socket.send_to(escvp_msg, format!("{b}:3629")).await;
    }
    
    // Method 2: EEMP registration probe (port 3620) - same as what Epson app sends
    // This is a simplified UDP probe; the real handshake is TCP but projectors
    // often respond to UDP probes on this port too
    let eemp_probe = b"EEMP0100\x00\x00\x00\x00\x02\x00\x00\x00\x30\x00\x00\x00";
    let _ = socket.send_to(eemp_probe, "255.255.255.255:3620").await;
    
    // Also try common projector subnet (192.168.88.x for direct Wi-Fi projectors)
    let _ = socket.send_to(escvp_msg, "192.168.88.255:3629").await;
    
    // Try current subnet broadcast too (covers Infrastructure mode projectors)
    let _ = socket.send_to(escvp_msg, "192.168.1.255:3629").await;
    let _ = socket.send_to(escvp_msg, "192.168.0.255:3629").await;
    let _ = socket.send_to(escvp_msg, "10.255.255.255:3629").await;

    let mut projectors = Vec::new();
    let mut seen_ips = std::collections::HashSet::new();
    let mut buf = [0u8; 2048];

    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        match tokio::time::timeout(Duration::from_millis(300), socket.recv_from(&mut buf)).await {
            Ok(Ok((len, addr))) => {
                let ip = addr.ip().to_string();
                if seen_ips.contains(&ip) { continue; }
                seen_ips.insert(ip.clone());

                let data = &buf[..len];
                
                // Try to extract projector name from response
                let name = extract_projector_name(data, &ip);
                
                projectors.push(ProjectorInfo {
                    name,
                    ip: ip.clone(),
                    model: "Epson Projector".into(),
                    status: "Available".into(),
                });
            }
            _ => {} // timeout or error, continue listening
        }
    }

    Ok(projectors)
}

/// Broadcast addresses of the directly connected IPv4 subnets. `255.255.255.255`
/// only leaves through the default-route interface, which is often not the one
/// the projector is on. Linux reads the routing table; elsewhere this is empty
/// and the fixed subnet guesses above still apply.
fn local_broadcasts() -> Vec<std::net::Ipv4Addr> {
    let mut out = Vec::new();
    #[cfg(target_os = "linux")]
    if let Ok(table) = std::fs::read_to_string("/proc/net/route") {
        for line in table.lines().skip(1) {
            let c: Vec<&str> = line.split_whitespace().collect();
            if c.len() < 8 || c[1] == "00000000" || c[0] == "lo" {
                continue;
            }
            if let (Ok(dst), Ok(mask)) = (u32::from_str_radix(c[1], 16), u32::from_str_radix(c[7], 16)) {
                if mask != 0 && mask != u32::MAX {
                    // Kernel stores both little-endian; OR-ing the inverted mask works the same.
                    let b = std::net::Ipv4Addr::from((dst | !mask).to_le_bytes());
                    if !out.contains(&b) {
                        out.push(b);
                    }
                }
            }
        }
    }
    out
}

/// Extract a human-readable projector name from discovery response data
fn extract_projector_name(data: &[u8], fallback_ip: &str) -> String {
    // Check for ESC/VP.net response
    if data.starts_with(b"ESC/VP.net") && data.len() > 16 {
        // Name is typically after the header bytes
        if let Some(name) = try_extract_ascii_name(&data[10..]) {
            if !name.is_empty() { return name; }
        }
    }
    
    // Check for EEMP response
    if data.starts_with(b"EEMP0100") && data.len() > 20 {
        // Scan the payload for ASCII name strings
        if let Some(name) = try_extract_ascii_name(&data[20..]) {
            if !name.is_empty() { return name; }
        }
    }
    
    // Try to find any readable ASCII name in the raw data
    if let Some(name) = try_extract_ascii_name(data) {
        if !name.is_empty() { return name; }
    }
    
    format!("Projector ({})", fallback_ip)
}

/// Scan raw bytes for a contiguous ASCII string (letters, digits, spaces, hyphens)
fn try_extract_ascii_name(data: &[u8]) -> Option<String> {
    let mut best_name = String::new();
    let mut current = String::new();
    
    for &b in data {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b' ' {
            current.push(b as char);
        } else {
            if current.len() > best_name.len() && current.len() >= 3 {
                best_name = current.clone();
            }
            current.clear();
        }
    }
    if current.len() > best_name.len() && current.len() >= 3 {
        best_name = current;
    }
    
    let trimmed = best_name.trim().to_string();
    if trimmed.len() >= 3 { Some(trimmed) } else { None }
}


/// Connects to a specific Wi-Fi network using OS-native tools.
#[tauri::command]
#[allow(unreachable_code)]
async fn connect_to_wifi(ssid: String, password: Option<String>, state: tauri::State<'_, AppState>) -> Result<bool, String> {
    println!("Connecting to network: {} (password provided: {})", ssid, password.is_some());

    // Remember where we came from so casting can put it back (only the first
    // switch: a reconnect must not overwrite it with the projector itself).
    {
        let mut prev = state.prev_wifi.lock().await;
        if prev.is_none() {
            *prev = current_wifi_id();
        }
    }

    // ── macOS ── `nmcli` is unavailable; use networksetup on the detected Wi-Fi port.
    #[cfg(target_os = "macos")]
    {
        let device = macos_wifi_device().unwrap_or_else(|| "en0".to_string());
        let mut args: Vec<String> =
            vec!["-setairportnetwork".to_string(), device, ssid.clone()];
        if let Some(ref pwd) = password {
            if !pwd.is_empty() {
                args.push(pwd.clone());
            }
        }

        let output = hidden_cmd("networksetup")
            .args(&args)
            .output()
            .map_err(|e| format!("Failed to execute networksetup: {}", e))?;

        // networksetup frequently exits 0 even on failure, printing the error to stdout.
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let lc = combined.to_lowercase();
        if output.status.success()
            && !lc.contains("could not")
            && !lc.contains("error")
            && !lc.contains("failed to join")
        {
            return Ok(true);
        }
        return Err(if combined.trim().is_empty() {
            "Failed to connect. Please check your password and try again.".to_string()
        } else {
            combined.trim().to_string()
        });
    }

    if cfg!(target_os = "windows") {
        // If a password was provided, create a temporary XML profile
        if let Some(ref pwd) = password {
            if !pwd.is_empty() {
                let profile_xml = format!(
                    r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
    <name>{ssid}</name>
    <SSIDConfig>
        <SSID>
            <name>{ssid}</name>
        </SSID>
    </SSIDConfig>
    <connectionType>ESS</connectionType>
    <connectionMode>auto</connectionMode>
    <MSM>
        <security>
            <authEncryption>
                <authentication>WPA2PSK</authentication>
                <encryption>AES</encryption>
                <useOneX>false</useOneX>
            </authEncryption>
            <sharedKey>
                <keyType>passPhrase</keyType>
                <protected>false</protected>
                <keyMaterial>{pwd}</keyMaterial>
            </sharedKey>
        </security>
    </MSM>
</WLANProfile>"#,
                    ssid = xml_escape(&ssid),
                    pwd = xml_escape(pwd)
                );

                // Write profile to a temp file
                let temp_dir = std::env::temp_dir();
                let profile_path = temp_dir.join("libremp_wifi_profile.xml");
                std::fs::write(&profile_path, &profile_xml)
                    .map_err(|e| format!("Failed to write Wi-Fi profile: {}", e))?;

                // Add the profile
                let add_output = hidden_cmd("netsh")
                    .args(["wlan", "add", "profile", &format!("filename={}", profile_path.display())])
                    .output()
                    .map_err(|e| format!("Failed to add Wi-Fi profile: {}", e))?;

                // Clean up temp file
                let _ = std::fs::remove_file(&profile_path);

                if !add_output.status.success() {
                    let stderr = String::from_utf8_lossy(&add_output.stderr);
                    let stdout = String::from_utf8_lossy(&add_output.stdout);
                    return Err(format!("Failed to add Wi-Fi profile: {} {}", stdout, stderr));
                }
            }
        }

        // Now connect using the profile name (which matches the SSID)
        let connect_output = hidden_cmd("netsh")
            .args(["wlan", "connect", &format!("name={}", ssid)])
            .output()
            .map_err(|e| format!("Failed to connect: {}", e))?;

        if !connect_output.status.success() {
            let stderr = String::from_utf8_lossy(&connect_output.stderr);
            let stdout = String::from_utf8_lossy(&connect_output.stdout);
            return Err(format!("Connection command failed: {} {}", stdout.trim(), stderr.trim()));
        }

        // Wait and verify actual connection (up to 15 seconds)
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(15);
        let mut consecutive_disconnected: u32 = 0;
        
        // Reduced initial sleep to start polling sooner
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        
        while start.elapsed() < timeout {
            // Faster polling (500ms instead of 1200ms) for smoother progress feedback
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            
            // Check current connection status via netsh
            let status_output = hidden_cmd("netsh")
                .args(["wlan", "show", "interfaces"])
                .output();
            
            if let Ok(output) = status_output {
                let stdout = String::from_utf8_lossy(&output.stdout);
                
                let mut iface_state = String::new();
                let mut iface_ssid = String::new();
                
                for line in stdout.lines() {
                    let trimmed = line.trim();
                    if let Some(colon_pos) = trimmed.find(':') {
                        let key = trimmed[..colon_pos].trim();
                        let val = trimmed[colon_pos + 1..].trim();
                        
                        let key_lower = key.to_lowercase();
                        if key_lower == "state" || key_lower == "status" {
                            iface_state = val.to_lowercase();
                        } else if key_lower == "ssid" {
                            iface_ssid = val.to_string();
                        }
                    }
                }
                
                // Successfully connected to target SSID
                if iface_ssid == ssid 
                    && iface_state.contains("connected") 
                    && !iface_state.contains("disconnected") 
                {
                    return Ok(true);
                }
                
                // Track consecutive disconnected polls for fast failure detection
                if iface_state.contains("disconnected") {
                    consecutive_disconnected += 1;
                } else if !iface_state.contains("authenticating") && !iface_state.contains("connecting") {
                    // Reset if we are in some other state (like identifying or already connected to wrong ssid)
                    consecutive_disconnected = 0;
                }
                
                // Faster failure detection (5 polls @ 500ms = 2.5s)
                if consecutive_disconnected >= 5 {
                    return Err("Authentication failed. Please check your password and try again.".to_string());
                }
                
                // If still stuck authenticating after 7s, password is likely wrong
                if start.elapsed() > std::time::Duration::from_secs(7) 
                    && iface_state.contains("authenticating") 
                {
                    return Err("Authentication failed. The password appears to be incorrect.".to_string());
                }
            }
        }
        
        // Timeout reached — connection failed
        Err("Connection timed out. The password may be incorrect or the network is unreachable.".to_string())
    } else {
        // Linux: nmcli handles password automatically
        let mut args = vec!["dev", "wifi", "connect", &ssid];
        let pwd_str;
        if let Some(ref pwd) = password {
            if !pwd.is_empty() {
                pwd_str = pwd.clone();
                args.push("password");
                args.push(&pwd_str);
            }
        }
        
        let output = hidden_cmd("nmcli")
            .args(&args)
            .output()
            .map_err(|e| format!("Failed to connect: {}", e))?;

        if output.status.success() {
            Ok(true)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Connection failed: {}", stderr.trim()))
        }
    }
}


/// Spawns the Rust Epson streamer process to begin casting.
///
/// `ip` (e.g. from the QR code or LAN discovery) is tried first; the streamer
/// falls back to the default gateways and 192.168.88.1 on its own.
#[tauri::command]
async fn start_casting_async(ssid: String, password: String, ip: Option<String>, state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let mut child_guard = state.child_process.lock().await;
    // A streamer that already exited (crash, bad binary) must not block a new cast.
    if let Some(child) = child_guard.as_mut() {
        if matches!(child.try_wait(), Ok(None)) {
            return Err("Already streaming".into());
        }
        *child_guard = None;
    }

    let binary = find_streamer_binary().ok_or_else(|| {
        "Could not find the epson-streamer program. Run `cargo build --release` at the repository root first.".to_string()
    })?;

    // --give-up-after: exit (so the UI can say so) if the projector never answers.
    let mut args = vec!["--skip-wifi", "--stop-on-stdin-eof", "--give-up-after", "3", "--ssid", &ssid, "--password", &password];
    let ip = ip.filter(|ip| ip.parse::<std::net::Ipv4Addr>().is_ok());
    if let Some(ip) = &ip {
        args.extend(["--projector-ip", ip.as_str()]);
    }
    println!("[+] Spawning streamer: {:?} (ssid {}, ip {:?})", binary, ssid, ip);

    let child = hidden_cmd(&binary)
        .args(&args)
        // Closing stdin is the cross-platform "stop" signal (see stop_casting).
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to spawn streamer: {}", e))?;

    *child_guard = Some(child);
    Ok(true)
}

/// Locates the `epson-streamer` executable binary.
fn find_streamer_binary() -> Option<std::path::PathBuf> {
    // `epson-streamer.exe` on Windows; without the suffix the file is never found there.
    let name = format!("epson-streamer{}", std::env::consts::EXE_SUFFIX);
    let exe = std::env::current_exe().ok();
    let candidates = [
        // Installed next to the app.
        exe.as_ref().and_then(|p| p.parent()).map(|d| d.join(&name)),
        // Development: exe at <root>/frontend/src-tauri/target/{debug,release}/libremp-app.
        exe.as_ref()
            .and_then(|p| p.parent()?.parent()?.parent()?.parent()?.parent())
            .map(|root| root.join("target/release").join(&name)),
        // Development: relative to the src-tauri cwd.
        Some(std::path::Path::new("../../target/release").join(&name)),
    ];

    for path in candidates.into_iter().flatten() {
        if path.exists() {
            return Some(std::fs::canonicalize(&path).unwrap_or(path));
        }
    }

    // Try finding via PATH
    #[cfg(not(windows))]
    if let Ok(output) = hidden_cmd("which").arg("epson-streamer").output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                return Some(std::path::PathBuf::from(path_str));
            }
        }
    }

    None
}

/// Stops casting. Closing the streamer's stdin makes it send the EasyMP
/// disconnect, so the projector frees the session at once; kill only if it hangs.
#[tauri::command]
async fn stop_casting(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let mut child_guard = state.child_process.lock().await;
    if let Some(mut child) = child_guard.take() {
        drop(child.stdin.take());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while matches!(child.try_wait(), Ok(None)) && std::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        if matches!(child.try_wait(), Ok(None)) {
            let _ = child.kill();
            let _ = child.wait();
        }
        println!("[+] Streamer stopped");
    }
    drop(child_guard);

    if let Some(id) = state.prev_wifi.lock().await.take() {
        println!("[+] Restoring the previous Wi-Fi network");
        let _ = tokio::task::spawn_blocking(move || restore_wifi(&id)).await;
    }
    Ok(true)
}

/// Whether the streamer is still running (it exits on fatal errors).
#[tauri::command]
async fn casting_alive(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let mut child_guard = state.child_process.lock().await;
    let alive = child_guard.as_mut().is_some_and(|c| matches!(c.try_wait(), Ok(None)));
    if !alive {
        *child_guard = None;
    }
    Ok(alive)
}

/// True if an EasyMP projector accepts connections at `ip` right now — i.e. we
/// can cast over the current network without switching Wi-Fi.
#[tauri::command]
async fn probe_projector(ip: String) -> Result<bool, String> {
    let Ok(ip) = ip.parse::<std::net::Ipv4Addr>() else {
        return Ok(false);
    };
    tokio::task::spawn_blocking(move || {
        std::net::TcpStream::connect_timeout(&(ip, 3620).into(), std::time::Duration::from_millis(800)).is_ok()
    })
    .await
    .map_err(|e| e.to_string())
}

/// Initializes and starts the Tauri application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // WebKitGTK's DMABUF renderer crashes (SIGSEGV in libnvidia-gpucomp / EGL)
    // on NVIDIA proprietary drivers. Disabling it forces a stable fallback path.
    // Set before the webview starts; harmless on non-NVIDIA / non-Linux systems,
    // and we respect an explicit user override.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            child_process: Arc::new(Mutex::new(None)),
            prev_wifi: Arc::new(Mutex::new(None)),
            cam_tx: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            scan_wifi_networks,
            discover_projectors,
            decode_projector_qr,
            camera_preview_frame,
            camera_capture,
            camera_scan,
            camera_stop,
            list_saved_projectors,
            save_projector,
            forget_projector,
            connect_to_wifi,
            start_casting_async,
            stop_casting,
            casting_alive,
            probe_projector
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

