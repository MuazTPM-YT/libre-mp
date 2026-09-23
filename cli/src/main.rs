//! epson-streamer: cast screen to Epson projector from terminal.

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use libremp_core::session::{self, CastOptions, FailKind};
use libremp_core::wifi;

// read flags, maybe pick wi-fi, cast till ctrl+c
fn main() {
    eprintln!("=== Epson EasyMP Rust Streamer ===\n");

    let args: Vec<String> = std::env::args().collect();
    let has_flag = |f: &str| args.iter().any(|a| a == f);
    let get_arg = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1)).cloned();

    let (prev_wifi, ssid, password) = if has_flag("--skip-wifi") {
        let ssid = get_arg("--ssid").unwrap_or_default();
        eprintln!("[*] CLI mode: skip-wifi, ssid={ssid}");
        (None, ssid, get_arg("--password").unwrap_or_default())
    } else {
        pick_wifi()
    };

    let opts = CastOptions {
        ssid,
        password,
        projector_ip: get_arg("--projector-ip").and_then(|v| v.parse().ok()),
        keyword: get_arg("--keyword"),
        give_up_after: get_arg("--give-up-after").and_then(|v| v.parse().ok()),
        full_frames_only: has_flag("--full-frames"),
    };

    // ctrl+c / sigterm drop flag, so we say goodbye to projector
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || r.store(false, Ordering::Relaxed)).expect("Error setting Ctrl+C handler");

    let result = session::run(&opts, &running, &mut |_| {});
    if let Some(id) = prev_wifi {
        eprintln!("[*] Restoring Wi-Fi...");
        wifi::restore(&id);
    }
    if let Err(e) = result {
        eprintln!("[-] {e}");
        std::process::exit(if e.kind == FailKind::Capture { 3 } else { 2 });
    }
}

// list networks, ask which + password, join. gives old wi-fi id, ssid, password
fn pick_wifi() -> (Option<String>, String, String) {
    let prev = wifi::current_id();
    if let Some(p) = &prev {
        eprintln!("[*] Currently connected to: {p}");
    }
    eprintln!("Scanning for Wi-Fi networks... Please wait.\n");
    let networks = wifi::scan().unwrap_or_else(|e| {
        eprintln!("[-] {e}");
        Vec::new()
    });
    if networks.is_empty() {
        eprintln!("[-] No Wi-Fi networks found. Skipping Wi-Fi setup.");
        return (None, String::new(), String::new());
    }

    eprintln!("Available Wi-Fi Networks:");
    for (i, n) in networks.iter().enumerate() {
        let star = if n.is_projector { " ★" } else { "" };
        eprintln!("[{}] {} (Signal: {}%, Security: {}){star}", i + 1, n.ssid, n.signal, n.security);
    }
    let choice: usize = ask("\nSelect a network to connect to (or 0 to skip): ").parse().unwrap_or(0);
    let Some(net) = choice.checked_sub(1).and_then(|i| networks.get(i)) else {
        eprintln!("[*] Skipping Wi-Fi setup.");
        return (None, String::new(), String::new());
    };

    eprintln!("\n[*] Selected network: '{}'", net.ssid);
    let password = ask("    Wi-Fi Password (WPA2): ");
    if password.is_empty() && net.security != "Open" {
        eprintln!("[-] No password entered.");
        std::process::exit(1);
    }
    match wifi::connect(&net.ssid, Some(&password)) {
        Ok(()) => eprintln!("[+] Successfully connected to '{}'!", net.ssid),
        Err(e) => {
            eprintln!("[-] {e}");
            std::process::exit(1);
        }
    }
    (prev, net.ssid.clone(), password)
}

// prompt on stderr, read one trimmed line
fn ask(prompt: &str) -> String {
    eprint!("{prompt}");
    io::stderr().flush().ok();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap_or(0);
    line.trim().to_string()
}
