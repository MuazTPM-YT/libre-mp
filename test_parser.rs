use regex::Regex;

fn main() {
    let stdout = "
Interface name : Wi-Fi
There are 6 networks currently visible.

SSID 1 : AJIET_WIFI
    Network type            : Infrastructure        
    Authentication          : Open
    Encryption              : None
    BSSID 1                 : f2:74:d7:05:d8:62
         Signal             : 80%

SSID 6 : AJIETRCHLAB-fE8DSypQz51AR2Q5 11
    Network type            : Infrastructure        
    Authentication          : WPA2-Personal
    Encryption              : CCMP
    BSSID 1                 : ae:19:8e:89:83:a7     
         Signal             : 75%
";

    // Line-by-line state machine parser
    let mut current_ssid = String::new();
    let mut current_security = String::from("Open");
    let mut current_bssid = String::new();

    let ssid_line_re = Regex::new(r"^SSID\s+\d+\s*:\s*(.*)").unwrap();
    let auth_re = Regex::new(r"Authentication\s+:\s+([^\r\n]+)").unwrap();
    let bssid_re = Regex::new(r"BSSID\s+\d+\s+:\s+([0-9a-fA-F:]{17})").unwrap();
    let signal_re = Regex::new(r"Signal\s+:\s+(\d+)%").unwrap();

    let mut networks = Vec::new();

    for line in stdout.lines() {
        let trimmed = line.trim();

        if let Some(caps) = ssid_line_re.captures(trimmed) {
            current_ssid = caps[1].trim().to_string();
            current_security = String::from("Open");
            current_bssid = String::new();
            continue;
        }

        if current_ssid.is_empty() { continue; }

        if let Some(caps) = auth_re.captures(trimmed) {
            current_security = caps[1].trim().to_string();
            continue;
        }

        if let Some(caps) = bssid_re.captures(trimmed) {
            current_bssid = caps[1].to_string();
            continue;
        }

        if let Some(caps) = signal_re.captures(trimmed) {
            if let Ok(sig) = caps[1].parse::<u8>() {
                let bssid = if current_bssid.is_empty() {
                    format!("unknown-{}", networks.len())
                } else {
                    current_bssid.clone()
                };
                networks.push((current_ssid.clone(), bssid, sig, current_security.clone()));
                current_bssid = String::new();
            }
            continue;
        }
    }

    println!("{:#?}", networks);
}
