//! handshake bytes locked to real windows session (test.pcap); real mac swapped for fake

use std::net::Ipv4Addr;
use libremp_core::protocol::{
    auth_payload, auth_payload_v9, parse_registration_response, registration_payload,
    response_0x0108,
};

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

const MY_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 88, 2);
const PROJ_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 88, 1);

/// Windows -> projector, cmd 0x0002 (68 bytes: 20 header + 48 payload).
const WIN_REGISTRATION: &str = "45454d5030313030c0a858020200000030000000007f0000b0f8ef5314000000000000000000000000000000000000000000000000000000000000000000000000000000";

/// Projector -> Windows, cmd 0x0003 (name + MAC) followed by cmd 0x0015.
const WIN_REGISTRATION_RESP: &str = concat!(
    "45454d5030313030c0a85802030000004a0000000100000052455345415243484c414200000000000000000000000000000000000000000001000b0000010000080000000200000000010000000000000000000000000000000000000000",
    "45454d5030313030c0a8580215000000ce000000020000000001000000000000000000000000000000000000000000b0000001040b000000020650423030000003060004000300000406020000040003055807090800000010001000200020009002a00100000000070b0800000008000800100010007002a00100000000070d0800000008000800100010009002a0010000000000000800000008000800100010009002a00100000000070a0100010c022d0000000008089f03f0d5000000000a0a002f0202320401000000000b0100000d0a000104401f0000000000000e010001",
);

/// Windows -> projector, cmd 0x0101 (264 bytes).
const WIN_AUTH: &str = "45454d5030313030c0a8580201010000f40000000101000000380f000000000000ffffff0000000000020f0b0004000320200001ff00ff00ff00000810000000010e000002000000000100000000000000000000000000000000c0a85801a600000005000000380000000200000004000000c0a858020c00000004000000000000000100000004000000500043000b00000004000000000000001c00000000000000040000003600000001000000030000002a000000020000000001c0a8580152455345415243484c41420000000000000000000000000000000000000000000f00000004000000320000000d000000040000000200000026000000080000000010000000100000";

/// Windows -> projector, cmd 0x0108 (reply to the projector's 0x010E).
const WIN_0108: &str = "45454d5030313030c0a8580208010000480100000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001401000005000000380000000200000004000000c0a858020c00000004000000010000000100000004000000500043000b00000004000000000000001c0000000000000007000000440000000100000005000000380000000200000004000000c0a858020c00000004000000010000000100000004000000500043000b00000004000000000000001c0000000000000008000000800000000400000005000000380000000200000004000000c0a858020c00000004000000010000000100000004000000500043000b00000004000000010100001c00000000000000000000000c000000020000000400000002000000000000000c000000020000000400000003000000000000000c000000020000000400000004000000";

const MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x01];

#[test]
fn registration_matches_windows_byte_for_byte() {
    assert_eq!(registration_payload(MY_IP), unhex(WIN_REGISTRATION));
}

#[test]
fn registration_response_yields_projector_name_and_mac() {
    let id = parse_registration_response(&unhex(WIN_REGISTRATION_RESP));
    assert_eq!(id.name.as_deref(), Some(&b"RESEARCHLAB"[..]));
    assert_eq!(id.mac, Some(MAC));
    // WIN_REGISTRATION_RESP is a v11 (0x0104 0b) blob
    assert_eq!(id.version, Some(0x0b));
}

// ── v9 dialect (windows EasyMP vs Epson PowerLite 4650; capture not in repo) ──

const V9_IP: Ipv4Addr = Ipv4Addr::new(10, 240, 62, 181);
const V9_PROJ: Ipv4Addr = Ipv4Addr::new(10, 240, 61, 255);
const V9_MAC: [u8; 6] = [0xb0, 0xe8, 0x92, 0xfd, 0xaf, 0xd4];

/// Projector -> client, cmd 0x0015 from the v9 capture (version byte 0x09 after 0x0104).
const V9_REGISTRATION_MAC: &str = "45454d50303130300af03eb515000000b9000000b0e892fdafd40000000000000000000000000000000000000000009b0000010409000000020650423030000003060004000300000406020000040003055807090800000010001000200020009002a00100000000070b0800000008000800100010009002a00100000000070d0800000008000800100010009002a0010000000000000800000008000800100010009002a00100000000060400000000070a0100010c00000000000008089f030008000000000b040001020d0e";

/// Client -> projector, the v9 login on TCP 3620 (cmd 0x0004, 20 header + 95 payload).
const V9_AUTH: &str = "45454d50303130300af03eb5040000005f00000001010000001c00000000000000ffff00000af001010201030004000320200001ff00ff00ff00000810000000010c0000b0e892fdafd4000000000000000000000000000000000af03dff1100000011000000000000000e0000000100000002";

#[test]
fn v9_registration_yields_version_9() {
    let id = parse_registration_response(&unhex(V9_REGISTRATION_MAC));
    assert_eq!(id.mac, Some(V9_MAC));
    assert_eq!(id.version, Some(0x09));
}

#[test]
fn v9_auth_matches_capture_byte_for_byte() {
    assert_eq!(auth_payload_v9(V9_IP, V9_PROJ, &V9_MAC, None), unhex(V9_AUTH));
}

#[test]
fn v9_keyword_only_fills_the_16_byte_slot_after_the_mac() {
    let plain = auth_payload_v9(V9_IP, V9_PROJ, &V9_MAC, None);
    let keyed = auth_payload_v9(V9_IP, V9_PROJ, &V9_MAC, Some("2270"));
    assert_eq!(plain.len(), keyed.len());
    // payload byte 54 = wire offset 74 (20 header + 48 prefix + 6 mac)
    assert_eq!(&keyed[74..78], b"2270");
    assert!(keyed[78..90].iter().all(|&b| b == 0), "keyword slot stays zero-padded");
    assert_eq!(plain[..74], keyed[..74]);
    assert_eq!(plain[90..], keyed[90..]);
}

#[test]
fn auth_matches_windows_byte_for_byte() {
    assert_eq!(auth_payload(MY_IP, PROJ_IP, &MAC, b"RESEARCHLAB", None), unhex(WIN_AUTH));
}

#[test]
fn auth_tag_0x0b_does_not_depend_on_name_length() {
    // 0x0B at byte 142 is tlv tag, not name length (len RESEARCHLAB = 11 hid it)
    let p = auth_payload(MY_IP, PROJ_IP, &MAC, b"EBC0E9E5", None);
    assert_eq!(p.len(), 264);
    assert_eq!(&p[142..146], &[0x0b, 0, 0, 0]);
    assert_eq!(&p[192..200], b"EBC0E9E5");
    assert!(p[200..224].iter().all(|&b| b == 0), "name must be zero-padded to 32 bytes");
}

#[test]
fn keyword_only_fills_the_field_after_the_mac() {
    let plain = auth_payload(MY_IP, PROJ_IP, &MAC, b"RESEARCHLAB", None);
    let keyed = auth_payload(MY_IP, PROJ_IP, &MAC, b"RESEARCHLAB", Some("2270"));
    assert_eq!(plain.len(), keyed.len());
    assert_eq!(&keyed[74..78], b"2270");
    assert!(keyed[78..90].iter().all(|&b| b == 0), "keyword field stays zero-padded");
    // Nothing outside the 16-byte field may move.
    assert_eq!(plain[..74], keyed[..74]);
    assert_eq!(plain[90..], keyed[90..]);
}

#[test]
fn response_0x0108_matches_windows_and_rewrites_local_ip() {
    assert_eq!(response_0x0108(MY_IP), unhex(WIN_0108));
    let r = response_0x0108(Ipv4Addr::new(10, 0, 0, 5));
    assert!(!r.windows(4).any(|w| w == [192, 168, 88, 2]), "stale baked IP remains");
    assert!(r.windows(4).any(|w| w == [10, 0, 0, 5]), "new IP not written");
}
