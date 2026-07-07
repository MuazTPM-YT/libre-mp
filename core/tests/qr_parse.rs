//! Verifies the Epson Quick Connect QR parser against the real payload decoded
//! from the RESEARCHLAB projector's on-screen QR code (XOR-0xE5 obfuscated).

use std::net::Ipv4Addr;
use libremp_core::qr::{parse, deobfuscate};

/// The exact 55-byte payload read off the projector's QR by quircs.
const RAW: [u8; 55] = [
    0xd2, 0xc7, 0xe7, 0x25, 0x4d, 0xbd, 0xe4, 0x41, 0x32, 0xd9, 0x28, 0x4a, 0xa0, 0xe9,
    0xa4, 0xd1, 0xa1, 0xd2, 0xd6, 0xa6, 0xa6, 0xa1, 0xa4, 0xa3, 0xd1, 0xd0, 0xfe, 0xb7,
    0xa0, 0xb6, 0xa0, 0xa4, 0xb7, 0xa6, 0xad, 0xa9, 0xa4, 0xa7, 0xc8, 0x83, 0xa0, 0xdd,
    0xa1, 0xb6, 0x9c, 0x95, 0xb4, 0x9f, 0xd0, 0xd4, 0xa4, 0xb7, 0xd7, 0xb4, 0x65,
];

#[test]
fn deobfuscation_reveals_ascii() {
    let d = deobfuscate(&RAW);
    // The SSID string must appear verbatim once XOR-0xE5 is undone.
    let needle = b"RESEARCHLAB-fE8D";
    assert!(
        d.windows(needle.len()).any(|w| w == needle),
        "expected SSID prefix in de-obfuscated payload"
    );
}

#[test]
fn parses_all_fields() {
    let qr = parse(&RAW).expect("should parse as an Epson QR");

    assert_eq!(qr.ip, Ipv4Addr::new(192, 168, 88, 1), "Direct-mode IP");
    assert_eq!(qr.mac, [0xa4, 0xd7, 0x3c, 0xcd, 0xaf, 0x45], "raw MAC");
    assert_eq!(qr.mac_hex(), "a4d73ccdaf45", "MAC as auth-token hex");

    assert_eq!(qr.fields.len(), 2, "password + SSID fields");
    // Confirmed against the iPhone Wi-Fi settings: password is the uppercase MAC.
    assert_eq!(qr.wifi_password(), Some("A4D73CCDAF45"));
    assert_eq!(qr.ssid(), Some("RESEARCHLAB-fE8DSypQz51AR2Q"));
}

#[test]
fn rejects_non_epson_payload() {
    // A plain "WIFI:..." string is not an Epson record → None.
    assert!(parse(b"WIFI:S:foo;T:WPA;P:bar;;").is_none());
    assert!(parse(&[0x00, 0x01, 0x02]).is_none());
}
