//! Verifies the Epson Quick Connect QR parser against payloads decoded from TWO
//! real projectors — a wired-LAN model (RESEARCHLAB) and a wireless-only model
//! (EBC0E9E5). Their header layouts differ, so passing both proves the parser is
//! structure-tolerant rather than overfit to one sample.

use std::net::Ipv4Addr;
use libremp_core::qr::{parse, deobfuscate};

/// RESEARCHLAB (wired LAN). Password confirmed via the phone's Wi-Fi settings.
const RESEARCHLAB: [u8; 55] = [
    0xd2, 0xc7, 0xe7, 0x25, 0x4d, 0xbd, 0xe4, 0x41, 0x32, 0xd9, 0x28, 0x4a, 0xa0, 0xe9,
    0xa4, 0xd1, 0xa1, 0xd2, 0xd6, 0xa6, 0xa6, 0xa1, 0xa4, 0xa3, 0xd1, 0xd0, 0xfe, 0xb7,
    0xa0, 0xb6, 0xa0, 0xa4, 0xb7, 0xa6, 0xad, 0xa9, 0xa4, 0xa7, 0xc8, 0x83, 0xa0, 0xdd,
    0xa1, 0xb6, 0x9c, 0x95, 0xb4, 0x9f, 0xd0, 0xd4, 0xa4, 0xb7, 0xd7, 0xb4, 0x65,
];

/// EBC0E9E5 (wireless-only). Different header layout: MAC shifted by a byte.
const WIRELESS: [u8; 53] = [
    0xd0, 0xe7, 0xe7, 0x25, 0x4d, 0xbd, 0xe4, 0x63, 0xdd, 0xff, 0xb7, 0x25, 0x0c, 0x00,
    0xe9, 0xd6, 0xdd, 0xd4, 0xa4, 0xd0, 0xd7, 0xa6, 0xd5, 0xa0, 0xdc, 0xa0, 0xd0, 0xfd,
    0xa0, 0xa7, 0xa6, 0xd5, 0xa0, 0xdc, 0xa0, 0xd0, 0xc8, 0xa0, 0xa0, 0xdd, 0xd4, 0x83,
    0xac, 0x88, 0xa0, 0x81, 0x87, 0xd5, 0xdc, 0xaa, 0x80, 0xa3, 0x65,
];

#[test]
fn deobfuscation_reveals_ascii() {
    assert!(deobfuscate(&RESEARCHLAB)
        .windows(11)
        .any(|w| w == b"RESEARCHLAB"));
    assert!(deobfuscate(&WIRELESS).windows(8).any(|w| w == b"EBC0E9E5"));
}

#[test]
fn parses_wired_researchlab() {
    let qr = parse(&RESEARCHLAB).expect("parse RESEARCHLAB");
    assert_eq!(qr.ip, Ipv4Addr::new(192, 168, 88, 1));
    assert_eq!(qr.wifi_password(), Some("A4D73CCDAF45"));
    assert_eq!(qr.ssid(), Some("RESEARCHLAB-fE8DSypQz51AR2Q"));
    assert_eq!(qr.mac_hex().as_deref(), Some("a4d73ccdaf45"));
    assert_eq!(qr.mac_bytes(), Some([0xa4, 0xd7, 0x3c, 0xcd, 0xaf, 0x45]));
}

#[test]
fn parses_wireless_only_projector() {
    let qr = parse(&WIRELESS).expect("parse EBC0E9E5");
    assert_eq!(qr.ip, Ipv4Addr::new(192, 168, 88, 1));
    assert_eq!(qr.wifi_password(), Some("381A52C0E9E5"));
    // On-screen SSID line is truncated; the QR carries the full value.
    assert!(qr.ssid().unwrap().starts_with("EBC0E9E5-"), "ssid: {:?}", qr.ssid());
    assert_eq!(qr.mac_hex().as_deref(), Some("381a52c0e9e5"));
    assert_eq!(qr.mac_bytes(), Some([0x38, 0x1a, 0x52, 0xc0, 0xe9, 0xe5]));
}

#[test]
fn rejects_non_epson_payload() {
    assert!(parse(b"WIFI:S:foo;T:WPA;P:bar;;").is_none());
    assert!(parse(&[0x00, 0x01, 0x02]).is_none());
}
