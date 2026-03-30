use std::process::Command;
use regex::Regex;

fn main() {
    let output = Command::new("netsh")
        .args(["wlan", "show", "networks", "mode=bssid"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    let auth_re = Regex::new(r"Authentication\s+:\s+([^\r\n]+)").unwrap();
    
    let blocks: Vec<&str> = stdout.split("SSID ").collect();
    for block in blocks.iter().skip(1) {
        let lines: Vec<&str> = block.lines().collect();
        if lines.is_empty() { continue; }
        
        let ssid_line = lines[0];
        let ssid_parts: Vec<&str> = ssid_line.splitn(2, ':').collect();
        if ssid_parts.len() != 2 { continue; }
        
        let ssid = ssid_parts[1].trim().to_string();
        if ssid.is_empty() { continue; }

        let mut security = String::from("Open");
        for line in &lines {
            if let Some(caps) = auth_re.captures(line) {
                security = caps[1].trim().to_string();
                break;
            }
        }

        for line in &lines {
            if line.contains("Signal") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() == 2 {
                    let sig_str = parts[1].trim().trim_end_matches('%');
                    if let Ok(sig_val) = sig_str.parse::<u8>() {
                        println!("Parsed Network -> SSID: '{}', Security: '{}', Signal: {}%", ssid, security, sig_val);
                        break;
                    }
                }
            }
        }
    }
}
