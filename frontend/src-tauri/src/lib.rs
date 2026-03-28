use serde::{Deserialize, Serialize};
use std::process::Command;
use regex::Regex;
use lazy_static::lazy_static;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod protocol;

pub struct AppState {
    pub streamer: Arc<Mutex<Option<protocol::streamer::VideoStreamer>>>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub ssid: Option<String>,
    pub ip_address: Option<String>,
}

lazy_static! {
    static ref SSID_RE: Regex = Regex::new(r"(?m)^[^B]*SSID\s+\d+\s+:\s+(.*)").unwrap();
    static ref BSSID_RE: Regex = Regex::new(r"BSSID\s+\d+\s+:\s+([0-9a-fA-F:]{17})").unwrap();
    static ref SIGNAL_RE: Regex = Regex::new(r"Signal\s+:\s+(\d+)%").unwrap();
    static ref AUTH_RE: Regex = Regex::new(r"Authentication\s+:\s+([^\r\n]+)").unwrap();
}

#[tauri::command]
async fn scan_wifi_networks() -> Result<Vec<WifiNetwork>, String> {
    if !cfg!(target_os = "windows") {
        return Ok(vec![
            WifiNetwork { ssid: "EPSON_Projector_CE1A4".into(), bssid: "00:11:22:33:44:55".into(), signal: 95, security: "Open".into(), is_projector: true },
            WifiNetwork { ssid: "Home_WiFi_5G".into(), bssid: "AA:BB:CC:DD:EE:FF".into(), signal: 82, security: "WPA2".into(), is_projector: false },
        ]);
    }

    let output = Command::new("netsh")
        .args(["wlan", "show", "networks", "mode=bssid"])
        .output()
        .map_err(|e| format!("Failed to execute netsh: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut networks = Vec::new();
    
    // Line-by-line state machine parser
    let mut current_ssid = String::new();
    let mut current_security = String::from("Open");
    let mut current_bssid = String::new();

    let ssid_line_re = Regex::new(r"^SSID\s+\d+\s*:\s*(.*)").unwrap();

    for line in stdout.lines() {
        let trimmed = line.trim();

        // New SSID block starts
        if let Some(caps) = ssid_line_re.captures(trimmed) {
            current_ssid = caps[1].trim().to_string();
            current_security = String::from("Open");
            current_bssid = String::new();
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
            current_bssid = caps[1].to_string();
            continue;
        }

        // Signal line — this completes a network entry
        if let Some(caps) = SIGNAL_RE.captures(trimmed) {
            if let Ok(sig) = caps[1].parse::<u8>() {
                let bssid = if current_bssid.is_empty() {
                    format!("unknown-{}", networks.len())
                } else {
                    current_bssid.clone()
                };
                networks.push(WifiNetwork {
                    ssid: current_ssid.clone(),
                    bssid,
                    signal: sig,
                    security: current_security.clone(),
                    is_projector: current_ssid.to_lowercase().contains("epson"),
                });
                // Reset BSSID for next entry within same SSID block
                current_bssid = String::new();
            }
            continue;
        }
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

#[tauri::command]
async fn discover_projectors() -> Result<Vec<ProjectorInfo>, String> {
    use tokio::net::UdpSocket;
    use std::time::Duration;

    let socket = UdpSocket::bind("0.0.0.0:0").await.map_err(|e| e.to_string())?;
    socket.set_broadcast(true).map_err(|e| e.to_string())?;

    // Epson iProjection uses multiple discovery methods:
    // 1. ESC/VP.net broadcast on port 3629
    // 2. EEMP protocol probe on port 3620
    
    // Method 1: ESC/VP.net discovery (port 3629)
    let escvp_msg = b"ESC/VP.net\x10\x03\x00\x00\x00\x00";
    let _ = socket.send_to(escvp_msg, "255.255.255.255:3629").await;
    
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


#[tauri::command]
async fn connect_to_wifi(ssid: String) -> Result<bool, String> {
    println!("Connecting to network: {}", ssid);
    
    if cfg!(target_os = "windows") {
        let output = Command::new("netsh")
            .args(["wlan", "connect", "name=", &ssid])
            .output()
            .map_err(|e| e.to_string())?;
            
        Ok(output.status.success())
    } else {
        std::thread::sleep(std::time::Duration::from_secs(2));
        Ok(true)
    }
}

#[tauri::command]
async fn get_connection_status() -> Result<ConnectionStatus, String> {
    Ok(ConnectionStatus {
        connected: false,
        ssid: None,
        ip_address: None,
    })
}

#[tauri::command]
async fn start_casting(ip: String, _state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let mut streamer = protocol::streamer::VideoStreamer::new(&ip, 30);
    
    // Attempt negotiation immediately
    if let Err(e) = streamer.start().await {
        return Err(format!("Failed to connect to projector: {:?}", e));
    }
    
    // If successful, store it or let it run.
    // However, streamer.start() loops forever! We need to spawn it in a tokio task.
    Ok(true)
}

#[tauri::command]
async fn start_casting_async(ip: String, state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let mut streamer_guard = state.streamer.lock().await;
    if streamer_guard.is_some() {
        return Err("Already casting".into());
    }

    let mut streamer = protocol::streamer::VideoStreamer::new(&ip, 15);
    // Note: To make this robust, we'd spawn the start() method in a tokio task, 
    // but we can spawn it here. We need the streamer to be Clone or Arc'd.
    // For now, let's just create a basic bridge logic.
    *streamer_guard = Some(streamer);
    
    // We will launch a separate background task for the actual stream
    Ok(true)
}

#[tauri::command]
async fn stop_casting(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let mut streamer_guard = state.streamer.lock().await;
    if let Some(streamer) = streamer_guard.take() {
        streamer.stop();
    }
    Ok(true)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            streamer: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            scan_wifi_networks,
            discover_projectors,
            connect_to_wifi,
            get_connection_status,
            start_casting_async,
            stop_casting
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
