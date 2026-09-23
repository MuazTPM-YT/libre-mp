//! Locks the control-channel handshake to the real Windows iProjection session
//! (the `test.pcap` capture: client 192.168.88.2 -> RESEARCHLAB at 192.168.88.1).
//! Every byte below was sent or received by Epson's own Windows client, so a
//! byte-identical match means we speak exactly what the projector expects.

use std::net::Ipv4Addr;
use libremp_core::protocol::{
    auth_payload, parse_registration_response, registration_payload, response_0x0108,
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
}

#[test]
fn auth_matches_windows_byte_for_byte() {
    assert_eq!(auth_payload(MY_IP, PROJ_IP, &MAC, b"RESEARCHLAB", None), unhex(WIN_AUTH));
}

#[test]
fn auth_tag_0x0b_does_not_depend_on_name_length() {
    // 0x0B at byte 142 is a TLV tag, not the name length. It happened to equal
    // len("RESEARCHLAB") = 11, which hid the bug; other names must not change it.
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
