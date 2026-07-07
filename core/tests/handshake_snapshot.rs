use std::net::Ipv4Addr;
use libremp_core::protocol::{registration_payload, auth_payload, response_0x0108};

#[test]
fn registration_payload_is_stable() {
    let p = registration_payload(Ipv4Addr::new(192, 168, 88, 2));
    // 8 magic + 4 ip + 20 fixed + 32 zero = 64 bytes
    assert_eq!(p.len(), 64);
    assert_eq!(&p[0..8], b"EEMP0100");
    assert_eq!(&p[8..12], &[192, 168, 88, 2]);
}

#[test]
fn auth_payload_encodes_mac_and_ssid_name() {
    // password is the projector's wired MAC (hex, no separators)
    let p = auth_payload(
        Ipv4Addr::new(192, 168, 88, 2),
        Ipv4Addr::new(192, 168, 88, 1),
        "b0f8ef531400",
        "RESEARCHLAB-abcdef123456",
    );
    assert_eq!(&p[0..8], b"EEMP0100");
    // The SSID prefix "RESEARCHLAB" (11 bytes) must appear as the padded name.
    let needle = b"RESEARCHLAB";
    assert!(p.windows(needle.len()).any(|w| w == needle), "ssid name not embedded");
}

#[test]
fn response_0x0108_rewrites_local_ip() {
    let r = response_0x0108(Ipv4Addr::new(10, 0, 0, 5));
    // The baked template IP 192.168.88.2 must be fully replaced by 10.0.0.5.
    assert!(!r.windows(4).any(|w| w == [192, 168, 88, 2]), "stale baked IP remains");
    assert!(r.windows(4).any(|w| w == [10, 0, 0, 5]), "new IP not written");
}
