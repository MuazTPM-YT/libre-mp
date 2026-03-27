use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

// ═══════════════════════════════════════════════════════════
//  Data Structures
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WifiNetwork {
    pub ssid: String,
    pub bssid: String,
    pub signal: i32,
    pub security: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecentEntry {
    pub ssid: String,
    pub bssid: String,
    pub timestamp: u64,
}

// ═══════════════════════════════════════════════════════════
//  Config directory helper
// ═══════════════════════════════════════════════════════════

fn config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let dir = PathBuf::from(home).join(".config").join("libre-mp");
    let _ = fs::create_dir_all(&dir);
    dir
}

// ═══════════════════════════════════════════════════════════
//  Tauri Commands
// ═══════════════════════════════════════════════════════════

/// Scan for WiFi networks using nmcli.
/// Returns a JSON array of WifiNetwork objects, filtered for projector-like SSIDs.
#[tauri::command]
fn scan_wifi() -> Vec<WifiNetwork> {
    let output = Command::new("nmcli")
        .args(["-t", "-f", "SSID,BSSID,SIGNAL,SECURITY", "dev", "wifi", "list", "--rescan", "yes"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut networks: Vec<WifiNetwork> = Vec::new();
            let mut seen_ssids = std::collections::HashSet::new();

            for line in stdout.lines() {
                let parts: Vec<&str> = line.splitn(4, ':').collect();
                if parts.len() >= 4 {
                    let ssid = parts[0].replace("\\:", ":").trim().to_string();
                    if ssid.is_empty() || seen_ssids.contains(&ssid) {
                        continue;
                    }
                    seen_ssids.insert(ssid.clone());

                    let bssid = parts[1].replace("\\:", ":").trim().to_string();
                    let signal: i32 = parts[2].trim().parse().unwrap_or(0);
                    let security = parts[3].trim().to_string();

                    networks.push(WifiNetwork {
                        ssid,
                        bssid,
                        signal,
                        security,
                    });
                }
            }

            // Sort by signal strength descending
            networks.sort_by(|a, b| b.signal.cmp(&a.signal));
            networks
        }
        Err(e) => {
            eprintln!("[libre-mp] nmcli scan failed: {}", e);
            Vec::new()
        }
    }
}

/// Connect to a WiFi network using nmcli.
#[tauri::command]
fn connect_projector(ssid: String, bssid: String) -> bool {
    // Try connecting by BSSID first for precision
    let result = Command::new("nmcli")
        .args(["dev", "wifi", "connect", &bssid])
        .output();

    match result {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            println!("[libre-mp] connect stdout: {}", stdout);
            if !stderr.is_empty() {
                eprintln!("[libre-mp] connect stderr: {}", stderr);
            }
            out.status.success()
        }
        Err(e) => {
            eprintln!("[libre-mp] connect failed: {}", e);
            // Fallback: try by SSID
            if let Ok(out2) = Command::new("nmcli")
                .args(["dev", "wifi", "connect", &ssid])
                .output()
            {
                return out2.status.success();
            }
            false
        }
    }
}

/// Disconnect from current WiFi.
#[tauri::command]
fn disconnect_projector() -> bool {
    let result = Command::new("nmcli")
        .args(["dev", "disconnect", "wlan0"])
        .output();

    match result {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

/// Get connection status.
#[tauri::command]
fn get_connection_status() -> String {
    let output = Command::new("nmcli")
        .args(["-t", "-f", "ACTIVE,SSID", "dev", "wifi"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if line.starts_with("yes:") {
                    let ssid = line.trim_start_matches("yes:").to_string();
                    return format!("{{\"status\":\"connected\",\"ssid\":\"{}\"}}", ssid);
                }
            }
            r#"{"status":"disconnected","ssid":""}"#.to_string()
        }
        Err(_) => r#"{"status":"disconnected","ssid":""}"#.to_string(),
    }
}

/// Load recent connections from disk.
#[tauri::command]
fn get_recent() -> String {
    let path = config_dir().join("recent.json");
    match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(_) => "[]".to_string(),
    }
}

/// Save recent connections to disk.
#[tauri::command]
fn save_recent(entries: String) -> bool {
    let path = config_dir().join("recent.json");
    fs::write(&path, entries).is_ok()
}

/// Load settings from disk.
#[tauri::command]
fn get_settings() -> String {
    let path = config_dir().join("settings.json");
    match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(_) => "{}".to_string(),
    }
}

/// Save settings to disk.
#[tauri::command]
fn save_settings(settings: String) -> bool {
    let path = config_dir().join("settings.json");
    fs::write(&path, settings).is_ok()
}

// ═══════════════════════════════════════════════════════════
//  App entry point
// ═══════════════════════════════════════════════════════════

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            scan_wifi,
            connect_projector,
            disconnect_projector,
            get_connection_status,
            get_recent,
            save_recent,
            get_settings,
            save_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
