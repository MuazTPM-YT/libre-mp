// epson quick connect qr parser: fake records in both known layouts, real ones from .env

use libremp_core::qr::{deobfuscate, parse};
use std::net::Ipv4Addr;

// build obfuscated record: len, hdr, ip, [0x86 if wireless], mac, pw field, ssid field, 0x80
fn record(wireless: bool, mac: [u8; 6], ssid: &str) -> Vec<u8> {
    let pw: String = mac.iter().map(|b| format!("{b:02X}")).collect();
    let mut d = vec![0, if wireless { 0x02 } else { 0x22 }, 0x02, 192, 168, 88, 1];
    if wireless {
        d.push(0x86);
    }
    d.extend_from_slice(&mac);
    d.push(12);
    d.extend_from_slice(pw.as_bytes());
    d.push(ssid.len() as u8);
    d.extend_from_slice(ssid.as_bytes());
    d.push(0x80);
    d[0] = d.len() as u8;
    d.iter().map(|b| b ^ 0xE5).collect()
}

// wired layout: mac right after ip
#[test]
fn parses_wired_layout() {
    let raw = record(false, [0x02, 0x11, 0x22, 0x33, 0x44, 0x55], "TESTLAB-aB3dE5fG7hJ9kL");
    assert!(deobfuscate(&raw).windows(7).any(|w| w == b"TESTLAB"));
    let qr = parse(&raw).expect("parse wired");
    assert_eq!(qr.ip, Ipv4Addr::new(192, 168, 88, 1));
    assert_eq!(qr.wifi_password(), Some("021122334455"));
    assert_eq!(qr.ssid(), Some("TESTLAB-aB3dE5fG7hJ9kL"));
    assert_eq!(qr.mac_hex().as_deref(), Some("021122334455"));
    assert_eq!(qr.mac_bytes(), Some([0x02, 0x11, 0x22, 0x33, 0x44, 0x55]));
}

// wireless-only layout: mac shifted one byte
#[test]
fn parses_wireless_layout() {
    let raw = record(true, [0x02, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE], "PROJ42-Zx9Yw8Vu7Ts6R");
    let qr = parse(&raw).expect("parse wireless");
    assert_eq!(qr.ip, Ipv4Addr::new(192, 168, 88, 1));
    assert_eq!(qr.wifi_password(), Some("02AABBCCDDEE"));
    assert_eq!(qr.ssid(), Some("PROJ42-Zx9Yw8Vu7Ts6R"));
    assert_eq!(qr.mac_bytes(), Some([0x02, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE]));
}

// plain wi-fi qr and junk are not epson
#[test]
fn rejects_non_epson_payload() {
    assert!(parse(b"WIFI:S:foo;T:WPA;P:bar;;").is_none());
    assert!(parse(&[0x00, 0x01, 0x02]).is_none());
}

// value from env var, else from workspace .env file
fn secret(key: &str) -> Option<String> {
    if let Ok(v) = std::env::var(key) {
        return Some(v).filter(|v| !v.is_empty());
    }
    let file = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../.env")).ok()?;
    file.lines()
        .filter_map(|l| l.split_once('='))
        .find(|(k, _)| k.trim() == key)
        .map(|(_, v)| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

// real projector payloads, skipped when .env lacks them
#[test]
fn parses_real_projectors_from_env() {
    for model in ["WIRED", "WIRELESS"] {
        let get = |f: &str| secret(&format!("LIBREMP_TEST_QR_{model}_{f}"));
        let (Some(hex), Some(pw), Some(ssid)) = (get("HEX"), get("PASSWORD"), get("SSID")) else {
            eprintln!("skip {model}: no real QR in .env");
            continue;
        };
        let raw: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
        let qr = parse(&raw).unwrap_or_else(|| panic!("parse real {model}"));
        assert_eq!(qr.ip, Ipv4Addr::new(192, 168, 88, 1));
        assert_eq!(qr.wifi_password(), Some(pw.as_str()));
        assert_eq!(qr.ssid(), Some(ssid.as_str()));
        assert_eq!(qr.mac_hex(), Some(pw.to_ascii_lowercase()));
    }
}
