use std::io::{self, Write};
use std::process::Command;




/// Detect if an SSID belongs to an Epson projector.
fn is_epson_projector(ssid: &str) -> bool {
    if ssid.starts_with("DIRECT-") && ssid.contains("EPSON") {
        return true;
    }
    if ssid.len() > 10 {
        let parts: Vec<&str> = ssid.rsplitn(2, '-').collect();
        if parts.len() == 2 && parts[0].len() >= 12 {
            let suffix = parts[0];
            let has_mixed = suffix.chars().any(|c| c.is_ascii_uppercase())
                && suffix.chars().any(|c| c.is_ascii_lowercase())
                && suffix.chars().any(|c| c.is_ascii_digit());
            if has_mixed {
                return true;
            }
        }
    }
    false
}

#[derive(Debug)]
struct WifiNetwork {
    ssid: String,
    signal: String,
    security: String,
    bssid: String,
}

/// Interactive: display networks and let user choose.
fn interactive_select(networks: &[WifiNetwork]) -> Option<usize> {
    if networks.is_empty() {
        eprintln!("[-] No Wi-Fi networks found!");
        return None;
    }

    eprintln!("Available Wi-Fi Networks:");
    for (i, net) in networks.iter().enumerate() {
        let star = if is_epson_projector(&net.ssid) {
            " ★"
        } else {
            ""
        };
        eprintln!(
            "[{}] {} (Signal: {}%, Security: {}){star}",
            i + 1, net.ssid, net.signal, net.security,
        );
    }

    eprint!("\nSelect a network to connect to (or 0 to skip): ");
    io::stderr().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap_or(0);
    let choice: usize = input.trim().parse().unwrap_or(0);

    if choice == 0 || choice > networks.len() {
        eprintln!("[*] Skipping Wi-Fi setup.");
        return None;
    }
    Some(choice - 1)
}

// ═══════════════════════════════════════════════════════════════════════════════
// LINUX (nmcli)
// ═══════════════════════════════════════════════════════════════════════════════

/// Scans for available Wi-Fi networks using `nmcli` on Linux.
#[cfg(target_os = "linux")]
fn scan_networks() -> Vec<WifiNetwork> {
    use std::time::Duration;

    let _ = Command::new("nmcli")
        .args(["device", "wifi", "rescan"])
        .output();
    std::thread::sleep(Duration::from_secs(2));

    let output = Command::new("nmcli")
        .args(["-t", "-f", "BSSID,SSID,SIGNAL,SECURITY", "device", "wifi", "list"])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };

    let mut networks = Vec::new();
    let mut seen_ssids = std::collections::HashSet::new();

    for line in output.lines() {
        let unescaped = line.replace("\\:", "§");
        let fields: Vec<&str> = unescaped.split(':').collect();
        if fields.len() < 4 { continue; }

        let bssid = fields[0].replace('§', ":");
        let ssid = fields[1].replace('§', ":");
        let signal = fields[2].to_string();
        let security = fields[3..].join(" ").replace('§', ":");

        if ssid.is_empty() || !seen_ssids.insert(ssid.clone()) { continue; }

        networks.push(WifiNetwork { ssid, signal, security, bssid });
    }

    networks.sort_by(|a, b| {
        let sa: i32 = a.signal.parse().unwrap_or(0);
        let sb: i32 = b.signal.parse().unwrap_or(0);
        sb.cmp(&sa)
    });

    networks
}

/// Connects to the specified Wi-Fi network using `nmcli` on Linux.
#[cfg(target_os = "linux")]
fn connect_to_network(net: &WifiNetwork, password: &str) -> bool {
    let _ = Command::new("nmcli")
        .args(["connection", "delete", &net.ssid])
        .output();

    eprintln!("Attempting to connect to '{}' (BSSID: {})...", net.ssid, net.bssid);

    let status = Command::new("nmcli")
        .args(["device", "wifi", "connect", &net.ssid, "password", password])
        .status();

    matches!(status, Ok(s) if s.success())
}


/// Retrieves the active Wi-Fi connection UUID using nmcli on Linux.
#[cfg(target_os = "linux")]
fn get_current_connection_id() -> Option<String> {
    Command::new("nmcli")
        .args(["-t", "-f", "UUID", "connection", "show", "--active"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.lines().next().unwrap_or("").to_string())
        .filter(|s| !s.is_empty())
}

/// Displays the currently connected Wi-Fi network to the user on Linux.
#[cfg(target_os = "linux")]
fn show_current_network() {
    if let Ok(output) = Command::new("nmcli")
        .args(["-t", "-f", "NAME", "connection", "show", "--active"])
        .output()
    {
        let name = String::from_utf8_lossy(&output.stdout);
        let name = name.lines().next().unwrap_or("None");
        eprintln!("[*] Currently connected to: {name}");
    }
}

/// Restores the original Wi-Fi connection disconnected during projector setup on Linux.
#[cfg(target_os = "linux")]
pub fn wifi_restore(orig_uuid: Option<String>) {
    if let Some(uuid) = orig_uuid {
        eprintln!("\n[*] Restoring Wi-Fi (background)...");
        let _ = Command::new("nmcli")
            .args(["connection", "up", &uuid])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        eprintln!("[+] Network restore initiated. Exiting.");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// macOS (networksetup / airport)
// ═══════════════════════════════════════════════════════════════════════════════

/// Scans for available Wi-Fi networks using the `airport` utility on macOS.
#[cfg(target_os = "macos")]
fn scan_networks() -> Vec<WifiNetwork> {
    // macOS airport scan
    let output = Command::new("/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport")
        .args(["-s"])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };

    let mut networks = Vec::new();
    let mut seen_ssids = std::collections::HashSet::new();

    for line in output.lines().skip(1) { // skip header
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }

        // airport -s output format: SSID BSSID RSSI CHANNEL HT CC SECURITY
        // Columns are fixed-width, SSID can have spaces
        // BSSID starts at a known pattern (xx:xx:xx:xx:xx:xx)
        let parts: Vec<&str> = trimmed.splitn(2, |c: char| c == ' ' || c == '\t').collect();
        if parts.len() < 2 { continue; }

        // Try to find BSSID pattern in the line
        let bssid_re = find_mac_in_line(trimmed);
        if bssid_re.is_none() { continue; }
        let bssid = bssid_re.unwrap();

        // Extract SSID (everything before BSSID)
        let bssid_pos = trimmed.find(&bssid).unwrap_or(0);
        let ssid = trimmed[..bssid_pos].trim().to_string();
        if ssid.is_empty() || !seen_ssids.insert(ssid.clone()) { continue; }

        // Extract RSSI (number after BSSID)
        let after_bssid = trimmed[bssid_pos + bssid.len()..].trim();
        let rssi: i32 = after_bssid.split_whitespace().next()
            .and_then(|s| s.parse().ok()).unwrap_or(-100);

        // Convert RSSI to percentage (rough: -30=100%, -90=0%)
        let signal = ((rssi + 90).clamp(0, 60) * 100 / 60).to_string();

        // Security: last field
        let security = after_bssid.split_whitespace().last().unwrap_or("None").to_string();

        networks.push(WifiNetwork { ssid, signal, security, bssid });
    }

    networks.sort_by(|a, b| {
        let sa: i32 = a.signal.parse().unwrap_or(0);
        let sb: i32 = b.signal.parse().unwrap_or(0);
        sb.cmp(&sa)
    });

    networks
}

/// Extracts a MAC address pattern from a line of text on macOS scan output.
#[cfg(target_os = "macos")]
fn find_mac_in_line(line: &str) -> Option<String> {
    // Find a MAC address pattern xx:xx:xx:xx:xx:xx in the line
    let chars: Vec<char> = line.chars().collect();
    for i in 0..chars.len().saturating_sub(16) {
        if chars.get(i+2) == Some(&':') && chars.get(i+5) == Some(&':')
            && chars.get(i+8) == Some(&':') && chars.get(i+11) == Some(&':')
            && chars.get(i+14) == Some(&':')
        {
            let mac: String = chars[i..i+17].iter().collect();
            if mac.len() == 17 {
                return Some(mac);
            }
        }
    }
    None
}

/// Connects to the specified Wi-Fi network using the `networksetup` utility on macOS.
#[cfg(target_os = "macos")]
fn connect_to_network(net: &WifiNetwork, password: &str) -> bool {
    eprintln!("Attempting to connect to '{}'...", net.ssid);

    let status = Command::new("networksetup")
        .args(["-setairportnetwork", "en0", &net.ssid, password])
        .status();

    matches!(status, Ok(s) if s.success())
}


/// Retrieves the active Wi-Fi connection SSID using `networksetup` on macOS.
#[cfg(target_os = "macos")]
fn get_current_connection_id() -> Option<String> {
    Command::new("networksetup")
        .args(["-getairportnetwork", "en0"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            s.strip_prefix("Current Wi-Fi Network: ")
                .map(|n| n.trim().to_string())
        })
        .filter(|s| !s.is_empty())
}

/// Displays the currently connected Wi-Fi network to the user on macOS.
#[cfg(target_os = "macos")]
fn show_current_network() {
    if let Some(name) = get_current_connection_id() {
        eprintln!("[*] Currently connected to: {name}");
    }
}

/// Restores the original Wi-Fi connection disconnected during projector setup on macOS.
#[cfg(target_os = "macos")]
pub fn wifi_restore(orig_uuid: Option<String>) {
    if let Some(ssid) = orig_uuid {
        eprintln!("\n[*] Restoring Wi-Fi (background)...");
        let _ = Command::new("networksetup")
            .args(["-setairportnetwork", "en0", &ssid])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        eprintln!("[+] Network restore initiated. Exiting.");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// WINDOWS (netsh wlan)
// ═══════════════════════════════════════════════════════════════════════════════

/// Scans for available Wi-Fi networks using `netsh wlan` on Windows.
#[cfg(target_os = "windows")]
fn scan_networks() -> Vec<WifiNetwork> {
    let output = Command::new("netsh")
        .args(["wlan", "show", "networks", "mode=bssid"])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };

    let mut networks = Vec::new();
    let mut seen_ssids = std::collections::HashSet::new();
    let mut current_ssid = String::new();
    let mut current_security = String::new();
    let mut current_bssid = String::new();
    let mut current_signal = String::new();

    for line in output.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("SSID") && trimmed.contains(":") && !trimmed.starts_with("BSSID") {
            if let Some(val) = trimmed.splitn(2, ':').nth(1) {
                current_ssid = val.trim().to_string();
            }
        } else if trimmed.starts_with("Authentication") || trimmed.starts_with("Authentifizierung") {
            if let Some(val) = trimmed.splitn(2, ':').nth(1) {
                current_security = val.trim().to_string();
            }
        } else if trimmed.starts_with("BSSID") {
            if let Some(val) = trimmed.splitn(2, ':').nth(1) {
                // BSSID has colons, so rejoin everything after first ':'
                let after_label = &trimmed[trimmed.find(':').unwrap_or(0)+1..];
                current_bssid = after_label.trim().to_string();
            }
        } else if trimmed.starts_with("Signal") {
            if let Some(val) = trimmed.splitn(2, ':').nth(1) {
                current_signal = val.trim().replace('%', "");
            }

            // We have a complete entry
            if !current_ssid.is_empty() && seen_ssids.insert(current_ssid.clone()) {
                networks.push(WifiNetwork {
                    ssid: current_ssid.clone(),
                    signal: current_signal.clone(),
                    security: current_security.clone(),
                    bssid: current_bssid.clone(),
                });
            }
        }
    }

    networks.sort_by(|a, b| {
        let sa: i32 = a.signal.parse().unwrap_or(0);
        let sb: i32 = b.signal.parse().unwrap_or(0);
        sb.cmp(&sa)
    });

    networks
}

/// Connects to the specified Wi-Fi network using a temporary XML profile on Windows.
#[cfg(target_os = "windows")]
fn connect_to_network(net: &WifiNetwork, password: &str) -> bool {
    eprintln!("Attempting to connect to '{}'...", net.ssid);

    // Create temp XML profile
    let profile_xml = format!(
        r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
    <name>{ssid}</name>
    <SSIDConfig><SSID><name>{ssid}</name></SSID></SSIDConfig>
    <connectionType>ESS</connectionType>
    <connectionMode>manual</connectionMode>
    <MSM><security>
        <authEncryption><authentication>WPA2PSK</authentication>
            <encryption>AES</encryption><useOneX>false</useOneX></authEncryption>
        <sharedKey><keyType>passPhrase</keyType><protected>false</protected>
            <keyMaterial>{password}</keyMaterial></sharedKey>
    </security></MSM>
</WLANProfile>"#,
        ssid = net.ssid, password = password
    );

    let profile_path = std::env::temp_dir().join("epson_wifi_profile.xml");
    if std::fs::write(&profile_path, &profile_xml).is_err() {
        return false;
    }

    let _ = Command::new("netsh")
        .args(["wlan", "add", "profile", &format!("filename={}", profile_path.display())])
        .output();

    let status = Command::new("netsh")
        .args(["wlan", "connect", &format!("name={}", net.ssid)])
        .status();

    let _ = std::fs::remove_file(&profile_path);
    matches!(status, Ok(s) if s.success())
}


/// Retrieves the active Wi-Fi connection SSID using `netsh wlan` on Windows.
#[cfg(target_os = "windows")]
fn get_current_connection_id() -> Option<String> {
    Command::new("netsh")
        .args(["wlan", "show", "interfaces"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            for line in s.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("Profile") || trimmed.starts_with("SSID") {
                    if let Some(val) = trimmed.splitn(2, ':').nth(1) {
                        let v = val.trim().to_string();
                        if !v.is_empty() {
                            return Some(v);
                        }
                    }
                }
            }
            None
        })
}

/// Displays the currently connected Wi-Fi network to the user on Windows.
#[cfg(target_os = "windows")]
fn show_current_network() {
    if let Some(name) = get_current_connection_id() {
        eprintln!("[*] Currently connected to: {name}");
    }
}

/// Restores the original Wi-Fi connection disconnected during projector setup on Windows.
#[cfg(target_os = "windows")]
pub fn wifi_restore(orig_uuid: Option<String>) {
    if let Some(ssid) = orig_uuid {
        eprintln!("\n[*] Restoring Wi-Fi (background)...");
        let _ = Command::new("netsh")
            .args(["wlan", "connect", &format!("name={}", ssid)])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        eprintln!("[+] Network restore initiated. Exiting.");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SHARED: wifi_connect (same flow on all platforms)
// ═══════════════════════════════════════════════════════════════════════════════

/// Interactive flow to scan, select, and connect to a projector Wi-Fi network.
pub fn wifi_connect() -> (Option<String>, String, String, String) {
    let current = get_current_connection_id();
    show_current_network();

    eprintln!("Scanning for Wi-Fi networks... Please wait.\n");
    let networks = scan_networks();

    let idx = match interactive_select(&networks) {
        Some(i) => i,
        None => return (current, String::new(), String::new(), String::new()),
    };

    let selected = &networks[idx];
    eprintln!();
    eprintln!("[*] Selected network: '{}'", selected.ssid);

    eprint!("    Wi-Fi Password (WPA2): ");
    io::stderr().flush().ok();
    let mut pw = String::new();
    io::stdin().read_line(&mut pw).unwrap_or(0);
    let password = pw.trim().to_string();

    if password.is_empty() {
        eprintln!("[-] No password entered.");
        std::process::exit(1);
    }

    if connect_to_network(selected, &password) {
        eprintln!("[+] Successfully connected to '{}'!", selected.ssid);
        return (current, selected.ssid.clone(), selected.bssid.clone(), password);
    }

    eprintln!("[-] Failed to connect to '{}'", selected.ssid);
    std::process::exit(1);
}
