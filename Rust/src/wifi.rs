use std::io::{self, Write};
use std::process::Command;
use std::time::Duration;

/// Known projector SSIDs and their passwords (MAC-based).
fn known_password(ssid: &str) -> Option<&'static str> {
    match ssid {
        "RESEARCHLAB-fE8DSypQz51AR2Q" => Some("A4D73CCDAF45"),
        _ if ssid.starts_with("DIRECT-") && ssid.contains("EPSON") => Some("A4D73CCDAF45"),
        _ => None,
    }
}

/// Detect if an SSID belongs to an Epson projector.
/// Epson projectors create Wi-Fi networks with recognizable patterns.
/// Password = BSSID MAC address (no colons, uppercase).
fn is_epson_projector(ssid: &str, _security: &str) -> bool {
    // Pattern 1: DIRECT-xx-EPSON-xxxxxx (Wi-Fi Direct mode)
    if ssid.starts_with("DIRECT-") && ssid.contains("EPSON") {
        return true;
    }
    // Pattern 2: SSIDs with long random suffix (projector-generated hotspot)
    // e.g., RESEARCHLAB-fE8DSypQz51AR2Q, A325-fC8DSypQye1AKdd
    if ssid.len() > 10 {
        let parts: Vec<&str> = ssid.rsplitn(2, '-').collect();
        if parts.len() == 2 && parts[0].len() >= 12 {
            // Has a long random suffix after a dash — likely a projector hotspot
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

fn scan_networks() -> Vec<WifiNetwork> {
    // Trigger a rescan
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
        // nmcli -t uses ':' as separator, but BSSID contains ':'
        // Format: AA\:BB\:CC\:DD\:EE\:FF:SSID:SIGNAL:SECURITY
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 4 {
            continue;
        }

        // BSSID is the first 6 colon-separated hex pairs (escaped with \)
        // In -t mode, colons in BSSID are escaped as \:
        // Let's use a different approach: split on unescaped colons
        let unescaped = line.replace("\\:", "§");
        let fields: Vec<&str> = unescaped.split(':').collect();
        if fields.len() < 4 {
            continue;
        }

        let bssid = fields[0].replace('§', ":");
        let ssid = fields[1].replace('§', ":");
        let signal = fields[2].to_string();
        let security = fields[3..].join(" ").replace('§', ":");

        if ssid.is_empty() || !seen_ssids.insert(ssid.clone()) {
            continue;
        }

        networks.push(WifiNetwork {
            ssid,
            signal,
            security,
            bssid,
        });
    }

    // Sort by signal strength descending
    networks.sort_by(|a, b| {
        let sa: i32 = a.signal.parse().unwrap_or(0);
        let sb: i32 = b.signal.parse().unwrap_or(0);
        sb.cmp(&sa)
    });

    networks
}

pub fn wifi_connect() -> Option<String> {
    // Get current connection UUID for later restoration
    let current = Command::new("nmcli")
        .args(["-t", "-f", "UUID", "connection", "show", "--active"])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8(o.stdout)
                .ok()
                .map(|s| s.lines().next().unwrap_or("").to_string())
        })
        .filter(|s| !s.is_empty());

    // Show current connection
    if let Ok(output) = Command::new("nmcli")
        .args(["-t", "-f", "NAME", "connection", "show", "--active"])
        .output()
    {
        let name = String::from_utf8_lossy(&output.stdout);
        let name = name.lines().next().unwrap_or("None");
        eprintln!("[*] Currently connected to: {name}");
    }

    eprintln!("Scanning for Wi-Fi networks... Please wait.\n");
    let networks = scan_networks();

    if networks.is_empty() {
        eprintln!("[-] No Wi-Fi networks found!");
        std::process::exit(1);
    }

    eprintln!("Available Wi-Fi Networks:");
    for (i, net) in networks.iter().enumerate() {
        let known = if known_password(&net.ssid).is_some()
            || is_epson_projector(&net.ssid, &net.security)
        {
            " ★"
        } else {
            ""
        };
        eprintln!(
            "[{}] {} (Signal: {}%, Security: {}){known}",
            i + 1,
            net.ssid,
            net.signal,
            net.security,
        );
    }

    eprint!("\nSelect a network to connect to (or 0 to skip): ");
    io::stderr().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap_or(0);
    let choice: usize = input.trim().parse().unwrap_or(0);

    if choice == 0 || choice > networks.len() {
        eprintln!("[*] Skipping Wi-Fi setup.");
        return current;
    }

    let selected = &networks[choice - 1];
    eprintln!();

    // Determine password
    let password = if let Some(pw) = known_password(&selected.ssid) {
        // 1. Exact match in our known projector database
        eprintln!("[*] Known projector detected! Using saved password.");
        pw.to_string()
    } else if is_epson_projector(&selected.ssid, &selected.security) {
        // 2. Auto-detect Epson projector → derive password from BSSID MAC
        let mac_pw = selected.bssid.replace(':', "").to_uppercase();
        eprintln!("[*] Epson projector detected! Deriving password from MAC: {mac_pw}");
        mac_pw
    } else {
        // 3. Unknown network → ask for password
        eprint!("Enter password for '{}': ", selected.ssid);
        io::stderr().flush().ok();
        let mut pw = String::new();
        io::stdin().read_line(&mut pw).unwrap_or(0);
        pw.trim().to_string()
    };

    // Delete any previous connection profile for this SSID
    let _ = Command::new("nmcli")
        .args(["connection", "delete", &selected.ssid])
        .output();

    eprintln!(
        "Attempting to connect to '{}' (BSSID: {})...",
        selected.ssid, selected.bssid
    );

    let status = Command::new("nmcli")
        .args([
            "device",
            "wifi",
            "connect",
            &selected.ssid,
            "password",
            &password,
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            eprintln!("[+] Successfully connected to '{}'!", selected.ssid);
            current
        }
        _ => {
            eprintln!("[-] Failed to connect to '{}'", selected.ssid);
            std::process::exit(1);
        }
    }
}

pub fn wifi_restore(orig_uuid: Option<String>) {
    if let Some(uuid) = orig_uuid {
        eprintln!("\n[*] Restoring Wi-Fi (background)...");
        // Spawn nmcli in background — don't block exit
        let _ = Command::new("nmcli")
            .args(["connection", "up", &uuid])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        eprintln!("[+] Network restore initiated. Exiting.");
    }
}
