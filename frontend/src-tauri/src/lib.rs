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
        let output = Command::new("nmcli")
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
                    let is_projector = {
                        let l = ssid.to_lowercase();
                        l.contains("epson") || l.contains("projector") || l.contains("direct-") || l.contains("display") || l.contains("cast")
                    };
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
                    is_projector: {
                        let l = current_ssid.to_lowercase();
                        l.contains("epson") || l.contains("projector") || l.contains("direct-") || l.contains("display") || l.contains("cast")
                    },
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
                    is_projector: {
                        let l = current_ssid.to_lowercase();
                        l.contains("epson") || l.contains("projector") || l.contains("direct-") || l.contains("display") || l.contains("cast")
                    },
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
async fn connect_to_wifi(ssid: String, password: Option<String>) -> Result<bool, String> {
    println!("Connecting to network: {} (password provided: {})", ssid, password.is_some());
    
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
                    ssid = ssid,
                    pwd = pwd
                );

                // Write profile to a temp file
                let temp_dir = std::env::temp_dir();
                let profile_path = temp_dir.join(format!("libremp_wifi_{}.xml", ssid.replace(' ', "_")));
                std::fs::write(&profile_path, &profile_xml)
                    .map_err(|e| format!("Failed to write Wi-Fi profile: {}", e))?;

                // Add the profile
                let add_output = Command::new("netsh")
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
        let connect_output = Command::new("netsh")
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
            let status_output = Command::new("netsh")
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
        
        let output = Command::new("nmcli")
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

#[tauri::command]
async fn get_connection_status() -> Result<ConnectionStatus, String> {
    Ok(ConnectionStatus {
        connected: false,
        ssid: None,
        ip_address: None,
    })
}


#[tauri::command]
async fn start_casting_async(ip: String, state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let mut streamer_guard = state.streamer.lock().await;
    if streamer_guard.is_some() {
        return Err("Already casting".into());
    }

    let streamer = protocol::streamer::VideoStreamer::new(&ip, 15);
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
