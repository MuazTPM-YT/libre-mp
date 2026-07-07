use serde::{Deserialize, Serialize};
use std::process::Command;
use regex::Regex;
use lazy_static::lazy_static;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub child_process: Arc<Mutex<Option<std::process::Child>>>,
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
    let out = Command::new("networksetup")
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
        let output = Command::new("system_profiler")
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


/// Connects to a specific Wi-Fi network using OS-native tools.
#[tauri::command]
#[allow(unreachable_code)]
async fn connect_to_wifi(ssid: String, password: Option<String>) -> Result<bool, String> {
    println!("Connecting to network: {} (password provided: {})", ssid, password.is_some());

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

        let output = Command::new("networksetup")
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

/// Retrieves the current Wi-Fi connection status (Stubbed implementation).
#[tauri::command]
async fn get_connection_status() -> Result<ConnectionStatus, String> {
    Ok(ConnectionStatus {
        connected: false,
        ssid: None,
        ip_address: None,
    })
}


/// Spawns the Rust Epson streamer process to begin casting.
#[tauri::command]
async fn start_casting_async(ssid: String, password: String, os_mode: u32, state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let mut child_guard = state.child_process.lock().await;
    if child_guard.is_some() {
        return Err("Already streaming".into());
    }

    // Find the epson-streamer binary
    let binary = find_streamer_binary()
        .ok_or_else(|| "Could not find epson-streamer binary. Run `cargo build --release` in the Rust/ directory first.".to_string())?;

    // Convert to absolute path so that setting current_dir doesn't break the executable lookup
    let binary = std::fs::canonicalize(&binary).unwrap_or(binary);

    // Derive the Rust/ directory from the binary path (binary is at Rust/target/release/epson-streamer)
    let rust_dir = binary.parent()   // target/release/
        .and_then(|p| p.parent())    // target/
        .and_then(|p| p.parent())    // Rust/
        .ok_or_else(|| "Could not determine Rust project directory".to_string())?;

    println!("[+] Spawning streamer: {:?}", binary);
    println!("[+] Working dir: {:?}", rust_dir);
    println!("[+] Args: --skip-wifi --ssid {} --os {}", ssid, os_mode);

    let child = Command::new(&binary)
        .current_dir(rust_dir)
        .args([
            "--skip-wifi",
            "--ssid", &ssid,
            "--password", &password,
            "--os", &os_mode.to_string(),
        ])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to spawn streamer: {}", e))?;

    *child_guard = Some(child);
    Ok(true)
}

/// Locates the `epson-streamer` executable binary.
fn find_streamer_binary() -> Option<std::path::PathBuf> {
    // The streamer now builds into the workspace-shared target/ dir (Rust/ was retired).
    let candidates = [
        // Development: relative to the src-tauri cwd → workspace root target/.
        std::path::PathBuf::from("../../target/release/epson-streamer"),
        std::path::PathBuf::from("../target/release/epson-streamer"),
        std::path::PathBuf::from("target/release/epson-streamer"),
        // Fallback: cli crate's own target dir if built in isolation.
        std::path::PathBuf::from("../../cli/target/release/epson-streamer"),
        // Relative to the running executable, walking up to the workspace root.
        std::env::current_exe().ok().and_then(|p| {
            // exe at <root>/frontend/src-tauri/target/{debug,release}/frontend
            p.parent()?.parent()?.parent()?.parent()?.parent()
                .map(|root| root.join("target/release/epson-streamer"))
        }).unwrap_or_default(),
    ];

    for path in &candidates {
        if path.exists() {
            return Some(path.clone());
        }
    }

    // Try finding via PATH
    if let Ok(output) = Command::new("which").arg("epson-streamer").output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                return Some(std::path::PathBuf::from(path_str));
            }
        }
    }

    None
}

/// Kills the active streaming process to stop casting.
#[tauri::command]
async fn stop_casting(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let mut child_guard = state.child_process.lock().await;
    if let Some(mut child) = child_guard.take() {
        let _ = child.kill();
        let _ = child.wait();
        println!("[+] Streamer process killed");
    }
    Ok(true)
}

/// Initializes and starts the Tauri application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            child_process: Arc::new(Mutex::new(None)),
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

