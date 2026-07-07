//! Decode & parse Epson iProjection "Quick Connect" QR codes.
//!
//! The QR payload is **not** the standard `WIFI:` schema — it is an Epson-specific
//! binary record, lightly obfuscated by XOR-ing every byte with `0xE5`. Once
//! de-obfuscated it is a short, length-prefixed structure carrying the
//! projector's Direct-mode IP, MAC, and SSID/credential strings.
//!
//! Reverse-engineered from a RESEARCHLAB projector. Decoded layout:
//!
//! ```text
//! 37 22            magic
//! 02               record type
//! c0 a8 58 01      IP        (192.168.88.1)
//! a4 d7 3c cd af45 MAC       (A4:D7:3C:CD:AF:45)
//! 0c <12 bytes>    len-prefixed ASCII MAC   "A4D73CCDAF45"
//! 1b <27 bytes>    len-prefixed SSID/creds  "RESEARCHLAB-fE8DSypQz51AR2Q"
//! 80               trailer
//! ```

use std::net::Ipv4Addr;

/// Epson's fixed obfuscation key for Quick Connect QR payloads.
const XOR_KEY: u8 = 0xE5;
/// Magic bytes that begin a decoded Epson QR record.
const MAGIC: [u8; 2] = [0x37, 0x22];

/// The structured contents of an Epson Quick Connect QR code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpsonQr {
    /// Projector Direct-mode (Quick Connect) IPv4 address.
    pub ip: Ipv4Addr,
    /// Projector MAC address (6 raw bytes).
    pub mac: [u8; 6],
    /// Length-prefixed ASCII fields after the header. Observed as
    /// `["<MAC hex>", "<SSID + passphrase>"]`.
    pub fields: Vec<String>,
}

impl EpsonQr {
    /// MAC as lowercase hex with no separators — used as the **EasyMP auth
    /// token** in the streaming handshake (hex-decoded, so case is irrelevant).
    pub fn mac_hex(&self) -> String {
        self.mac.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// The Wi-Fi passphrase (first length-prefixed field), returned **verbatim**
    /// — case is preserved because WPA passphrases are case-sensitive. On
    /// observed projectors this is the MAC in uppercase hex (e.g. `A4D73CCDAF45`),
    /// confirmed against the OS Wi-Fi settings.
    pub fn wifi_password(&self) -> Option<&str> {
        self.fields.first().map(|s| s.as_str())
    }

    /// The full network SSID (second length-prefixed field), e.g.
    /// `RESEARCHLAB-fE8DSypQz51AR2Q`. Note the projector's on-screen SSID line
    /// is often truncated; this is the untruncated value.
    pub fn ssid(&self) -> Option<&str> {
        self.fields.get(1).map(|s| s.as_str())
    }
}

/// Reverse Epson's XOR obfuscation.
pub fn deobfuscate(payload: &[u8]) -> Vec<u8> {
    payload.iter().map(|b| b ^ XOR_KEY).collect()
}

/// Parse a *raw* (still-obfuscated) QR payload into structured fields.
pub fn parse(raw_payload: &[u8]) -> Option<EpsonQr> {
    parse_deobfuscated(&deobfuscate(raw_payload))
}

/// Parse an already-de-obfuscated record.
pub fn parse_deobfuscated(d: &[u8]) -> Option<EpsonQr> {
    if d.len() < 13 || d[0..2] != MAGIC {
        return None;
    }
    let ip = Ipv4Addr::new(d[3], d[4], d[5], d[6]);
    let mut mac = [0u8; 6];
    mac.copy_from_slice(&d[7..13]);

    // Remaining bytes are a series of length-prefixed ASCII fields, terminated
    // by a 0x80 (or 0x00) trailer.
    let mut fields = Vec::new();
    let mut pos = 13;
    while pos < d.len() {
        let len = d[pos] as usize;
        if len == 0 || len == 0x80 || pos + 1 + len > d.len() {
            break;
        }
        if let Ok(s) = std::str::from_utf8(&d[pos + 1..pos + 1 + len]) {
            fields.push(s.to_string());
        }
        pos += 1 + len;
    }
    Some(EpsonQr { ip, mac, fields })
}

/// Decode a QR code from an 8-bit grayscale buffer and parse it as an Epson
/// record. Returns `None` if no Epson QR is present.
pub fn parse_from_luma(width: usize, height: usize, gray: &[u8]) -> Option<EpsonQr> {
    let mut quirc = quircs::Quirc::default();
    for code in quirc.identify(width, height, gray) {
        let code = match code {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Ok(decoded) = code.decode() {
            if let Some(parsed) = parse(&decoded.payload) {
                return Some(parsed);
            }
        }
    }
    None
}
