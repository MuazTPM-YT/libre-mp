//! OS Wi-Fi: scan, join, remember, go back. nmcli / netsh / networksetup.

use serde::Serialize;
use std::collections::HashMap;
use std::process::{Command, Output, Stdio};

// one visible network
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub bssid: String,
    pub signal: u8,
    pub security: String,
    pub is_projector: bool,
}

// guess projector from name. epson quick connect ssid = name-<long mixed suffix>
pub fn is_projector_ssid(ssid: &str) -> bool {
    let l = ssid.to_lowercase();
    if ["epson", "projector", "direct-", "display", "cast"].iter().any(|k| l.contains(k)) {
        return true;
    }
    ssid.rsplit_once('-').is_some_and(|(name, suffix)| {
        !name.is_empty()
            && suffix.len() >= 12
            && suffix.bytes().all(|b| b.is_ascii_alphanumeric())
            && suffix.bytes().any(|b| b.is_ascii_uppercase())
            && suffix.bytes().any(|b| b.is_ascii_lowercase())
            && suffix.bytes().any(|b| b.is_ascii_digit())
    })
}

// command that never flashes console window on windows
fn cmd(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

// stdout + stderr as one trimmed string
fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)).trim().to_string()
}

// blank or "--" security mean open network
fn open_if_blank(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() || s == "--" { "Open".into() } else { s.into() }
}

// one entry per ssid, strongest kept, strongest first
fn strongest_first(nets: Vec<WifiNetwork>) -> Vec<WifiNetwork> {
    let mut best: HashMap<String, WifiNetwork> = HashMap::new();
    for n in nets {
        if best.get(&n.ssid).is_none_or(|b| n.signal > b.signal) {
            best.insert(n.ssid.clone(), n);
        }
    }
    let mut v: Vec<_> = best.into_values().collect();
    v.sort_by(|a, b| b.signal.cmp(&a.signal).then_with(|| a.ssid.cmp(&b.ssid)));
    v
}

// split nmcli -t line on unescaped ':'
fn nmcli_fields(line: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut escaped = false;
    for c in line.chars() {
        let last = out.last_mut().unwrap();
        if escaped {
            last.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == ':' {
            out.push(String::new());
        } else {
            last.push(c);
        }
    }
    out
}

// read `nmcli -t -f SSID,BSSID,SECURITY,SIGNAL dev wifi list`
pub fn parse_nmcli(out: &str) -> Vec<WifiNetwork> {
    let nets = out
        .lines()
        .filter_map(|line| {
            let f = nmcli_fields(line.trim_end_matches('\r'));
            let ssid = f.first()?.clone();
            if f.len() < 4 || ssid.is_empty() || ssid == "--" {
                return None;
            }
            Some(WifiNetwork {
                is_projector: is_projector_ssid(&ssid),
                bssid: f[1].clone(),
                security: open_if_blank(&f[2]),
                signal: f[3].trim().parse().unwrap_or(0),
                ssid,
            })
        })
        .collect();
    strongest_first(nets)
}

// read `netsh wlan show networks mode=bssid`. one row per bssid signal line
pub fn parse_netsh_networks(out: &str) -> Vec<WifiNetwork> {
    let (mut ssid, mut security, mut bssid) = (String::new(), String::from("Open"), String::new());
    let mut nets = Vec::new();
    for line in out.lines() {
        let Some((key, val)) = line.split_once(':') else { continue };
        let (key, val) = (key.trim().to_lowercase(), val.trim());
        if key.starts_with("ssid ") {
            ssid = val.to_string();
            security = "Open".into();
            bssid.clear();
        } else if key.starts_with("authentication") || key.starts_with("authentifizierung") {
            security = open_if_blank(val);
        } else if key.starts_with("bssid ") {
            bssid = val.to_string();
        } else if key == "signal" && !ssid.is_empty() {
            nets.push(WifiNetwork {
                ssid: ssid.clone(),
                bssid: bssid.clone(),
                signal: val.trim_end_matches('%').trim().parse().unwrap_or(0).max(1),
                security: security.clone(),
                is_projector: is_projector_ssid(&ssid),
            });
        }
    }
    strongest_first(nets)
}

// one adapter block from `netsh wlan show interfaces`
#[derive(Debug, Default, PartialEq, Eq)]
pub struct NetshInterface {
    pub state: String,
    pub ssid: String,
    pub profile: String,
}

// read `netsh wlan show interfaces`. keys vary by locale, so match loosely
pub fn parse_netsh_interfaces(out: &str) -> Vec<NetshInterface> {
    let mut all: Vec<NetshInterface> = Vec::new();
    for line in out.lines() {
        let Some((key, val)) = line.split_once(':') else { continue };
        let (key, val) = (key.trim().to_lowercase(), val.trim().to_string());
        if key == "name" {
            all.push(NetshInterface::default());
        }
        let Some(cur) = all.last_mut() else { continue };
        if key == "state" || key == "status" {
            cur.state = val.to_lowercase();
        } else if key == "ssid" {
            cur.ssid = val;
        } else if key.starts_with("profil") {
            cur.profile = val;
        }
    }
    all
}

// wi-fi profile we sit on now, from netsh text
pub fn netsh_current(out: &str) -> Option<String> {
    parse_netsh_interfaces(out)
        .into_iter()
        .filter(|i| !i.state.contains("disconnected"))
        .find_map(|i| [i.profile, i.ssid].into_iter().find(|s| !s.is_empty()))
}

// macos rssi "-55 dBm / -90 dBm" to 0..100
fn macos_signal(field: &str) -> u8 {
    let rssi: i32 = field.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(-100);
    (2 * (rssi + 100)).clamp(0, 100) as u8
}

// macos security mode to same words as other os
fn macos_security(mode: &str) -> String {
    let m = mode.to_lowercase();
    if m.is_empty() || m.contains("none") {
        "Open".into()
    } else if m.contains("wep") {
        "WEP".into()
    } else {
        "WPA2".into()
    }
}

// read `system_profiler SPAirPortDataType -json` (airport cli gone since macos 14)
pub fn parse_system_profiler(json: &str) -> Vec<WifiNetwork> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else { return Vec::new() };
    let ifaces = v
        .pointer("/SPAirPortDataType/0/spairport_airport_interfaces")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    let mut nets = Vec::new();
    for iface in &ifaces {
        let others = iface.get("spairport_airport_other_local_wireless_networks").and_then(|x| x.as_array());
        let current = iface.get("spairport_current_network_information");
        for net in others.into_iter().flatten().chain(current) {
            let Some(ssid) = net.get("_name").and_then(|x| x.as_str()).filter(|s| !s.is_empty()) else { continue };
            let field = |k: &str| net.get(k).and_then(|x| x.as_str()).unwrap_or("");
            nets.push(WifiNetwork {
                ssid: ssid.to_string(),
                bssid: String::new(),
                signal: macos_signal(field("spairport_signal_noise")),
                security: macos_security(field("spairport_security_mode")),
                is_projector: is_projector_ssid(ssid),
            });
        }
    }
    strongest_first(nets)
}

// escape text for windows profile xml
#[cfg(windows)]
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

// ── Linux / BSD: NetworkManager ─────────────────────────────────────────────

// list networks around us
#[cfg(not(any(target_os = "macos", windows)))]
pub fn scan() -> Result<Vec<WifiNetwork>, String> {
    let out = cmd("nmcli")
        .args(["-t", "-f", "SSID,BSSID,SECURITY,SIGNAL", "dev", "wifi", "list"])
        .output()
        .map_err(|e| format!("Could not run nmcli: {e}"))?;
    if !out.status.success() {
        return Err(format!("Wi-Fi scan failed: {}", text(&out)));
    }
    Ok(parse_nmcli(&String::from_utf8_lossy(&out.stdout)))
}

// join network. stale saved profile with no key gets dropped once and retried
#[cfg(not(any(target_os = "macos", windows)))]
pub fn connect(ssid: &str, password: Option<&str>) -> Result<(), String> {
    let attempt = || {
        let mut c = cmd("nmcli");
        c.args(["dev", "wifi", "connect", ssid]);
        if let Some(pw) = password.filter(|p| !p.is_empty()) {
            c.args(["password", pw]);
        }
        c.output().map_err(|e| format!("Could not run nmcli: {e}"))
    };
    let mut out = attempt()?;
    if !out.status.success() && text(&out).contains("key-mgmt") {
        let _ = cmd("nmcli").args(["connection", "delete", "id", ssid]).output();
        out = attempt()?;
    }
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("Could not join {ssid}: {}", text(&out)))
    }
}

// uuid of wi-fi connection now active. wired and vpn skipped
#[cfg(not(any(target_os = "macos", windows)))]
pub fn current_id() -> Option<String> {
    let out = cmd("nmcli").args(["-t", "-f", "UUID,TYPE", "connection", "show", "--active"]).output().ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.rsplit_once(':'))
        .find(|(_, kind)| kind.contains("wireless"))
        .map(|(uuid, _)| uuid.to_string())
}

// command that brings old wi-fi back
#[cfg(not(any(target_os = "macos", windows)))]
fn restore_command(id: &str) -> Command {
    let mut c = cmd("nmcli");
    c.args(["connection", "up", "uuid", id]);
    c
}

// ── Windows: netsh wlan ─────────────────────────────────────────────────────

// list networks around us
#[cfg(windows)]
pub fn scan() -> Result<Vec<WifiNetwork>, String> {
    let out = cmd("netsh")
        .args(["wlan", "show", "networks", "mode=bssid"])
        .output()
        .map_err(|e| format!("Could not run netsh: {e}"))?;
    if !out.status.success() {
        return Err(format!("Wi-Fi scan failed: {}", text(&out)));
    }
    Ok(parse_netsh_networks(&String::from_utf8_lossy(&out.stdout)))
}

// join network: write temp profile if password, connect, wait till joined
#[cfg(windows)]
pub fn connect(ssid: &str, password: Option<&str>) -> Result<(), String> {
    if let Some(pw) = password.filter(|p| !p.is_empty()) {
        let xml = format!(
            r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
    <name>{ssid}</name>
    <SSIDConfig><SSID><name>{ssid}</name></SSID></SSIDConfig>
    <connectionType>ESS</connectionType>
    <connectionMode>manual</connectionMode>
    <MSM><security>
        <authEncryption><authentication>WPA2PSK</authentication><encryption>AES</encryption><useOneX>false</useOneX></authEncryption>
        <sharedKey><keyType>passPhrase</keyType><protected>false</protected><keyMaterial>{pw}</keyMaterial></sharedKey>
    </security></MSM>
</WLANProfile>"#,
            ssid = xml_escape(ssid),
            pw = xml_escape(pw)
        );
        let path = std::env::temp_dir().join("libremp_wifi_profile.xml");
        std::fs::write(&path, xml).map_err(|e| format!("Could not write the Wi-Fi profile: {e}"))?;
        let added = cmd("netsh").args(["wlan", "add", "profile", &format!("filename={}", path.display())]).output();
        let _ = std::fs::remove_file(&path);
        let added = added.map_err(|e| format!("Could not run netsh: {e}"))?;
        if !added.status.success() {
            return Err(format!("Could not add the Wi-Fi profile: {}", text(&added)));
        }
    }
    let out = cmd("netsh")
        .args(["wlan", "connect", &format!("name={ssid}")])
        .output()
        .map_err(|e| format!("Could not run netsh: {e}"))?;
    if !out.status.success() {
        return Err(format!("Could not join {ssid}: {}", text(&out)));
    }
    wait_until_joined(ssid)
}

// netsh connect return early. poll till on target ssid or clearly failed
#[cfg(windows)]
fn wait_until_joined(ssid: &str) -> Result<(), String> {
    use std::time::{Duration, Instant};
    let start = Instant::now();
    let mut dropped = 0u32;
    std::thread::sleep(Duration::from_millis(500));
    while start.elapsed() < Duration::from_secs(15) {
        std::thread::sleep(Duration::from_millis(500));
        let Ok(out) = cmd("netsh").args(["wlan", "show", "interfaces"]).output() else { continue };
        let ifaces = parse_netsh_interfaces(&String::from_utf8_lossy(&out.stdout));
        let state = ifaces.first().map(|i| i.state.clone()).unwrap_or_default();
        if ifaces.iter().any(|i| i.ssid == ssid && i.state.contains("connected") && !i.state.contains("disconnected")) {
            return Ok(());
        }
        if state.contains("disconnected") {
            dropped += 1;
        } else if !state.contains("authenticating") && !state.contains("connecting") {
            dropped = 0;
        }
        if dropped >= 5 || (start.elapsed() > Duration::from_secs(7) && state.contains("authenticating")) {
            return Err("Wi-Fi sign-in failed. Check the password and try again.".into());
        }
    }
    Err("Joining the network timed out. The password may be wrong, or the network is out of range.".into())
}

// profile name of wi-fi now active
#[cfg(windows)]
pub fn current_id() -> Option<String> {
    let out = cmd("netsh").args(["wlan", "show", "interfaces"]).output().ok()?;
    netsh_current(&String::from_utf8_lossy(&out.stdout))
}

// command that brings old wi-fi back
#[cfg(windows)]
fn restore_command(id: &str) -> Command {
    let mut c = cmd("netsh");
    c.args(["wlan", "connect", &format!("name={id}")]);
    c
}

// ── macOS: networksetup + system_profiler ───────────────────────────────────

// wi-fi device name. not always en0
#[cfg(target_os = "macos")]
fn wifi_device() -> String {
    let out = cmd("networksetup").arg("-listallhardwareports").output().ok();
    let text = out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    let mut is_wifi = false;
    for line in text.lines().map(str::trim) {
        if let Some(port) = line.strip_prefix("Hardware Port:") {
            is_wifi = port.contains("Wi-Fi") || port.contains("AirPort");
        } else if let Some(dev) = line.strip_prefix("Device:").filter(|_| is_wifi) {
            return dev.trim().to_string();
        }
    }
    "en0".into()
}

// list networks around us. needs location permission to see names
#[cfg(target_os = "macos")]
pub fn scan() -> Result<Vec<WifiNetwork>, String> {
    let out = cmd("system_profiler")
        .args(["SPAirPortDataType", "-json"])
        .output()
        .map_err(|e| format!("Could not run system_profiler: {e}"))?;
    Ok(parse_system_profiler(&String::from_utf8_lossy(&out.stdout)))
}

// join network. networksetup exit 0 even on fail, so read its words
#[cfg(target_os = "macos")]
pub fn connect(ssid: &str, password: Option<&str>) -> Result<(), String> {
    let mut c = cmd("networksetup");
    c.args(["-setairportnetwork", &wifi_device(), ssid]);
    if let Some(pw) = password.filter(|p| !p.is_empty()) {
        c.arg(pw);
    }
    let out = c.output().map_err(|e| format!("Could not run networksetup: {e}"))?;
    let said = text(&out);
    let lc = said.to_lowercase();
    if out.status.success() && !lc.contains("could not") && !lc.contains("error") && !lc.contains("failed to join") {
        Ok(())
    } else if said.is_empty() {
        Err(format!("Could not join {ssid}. Check the password and try again."))
    } else {
        Err(said)
    }
}

// ssid of wi-fi now active
#[cfg(target_os = "macos")]
pub fn current_id() -> Option<String> {
    let out = cmd("networksetup").args(["-getairportnetwork", &wifi_device()]).output().ok()?;
    String::from_utf8_lossy(&out.stdout)
        .split_once("Current Wi-Fi Network: ")
        .map(|(_, n)| n.trim().to_string())
        .filter(|n| !n.is_empty())
}

// command that brings old wi-fi back (keychain has its password)
#[cfg(target_os = "macos")]
fn restore_command(id: &str) -> Command {
    let mut c = cmd("networksetup");
    c.args(["-setairportnetwork", &wifi_device(), id]);
    c
}

// ── shared ──────────────────────────────────────────────────────────────────

// go back to old wi-fi. no wait, so app can quit while os reconnects
pub fn restore(id: &str) {
    let spawned = restore_command(id).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    if let Ok(mut child) = spawned {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}
