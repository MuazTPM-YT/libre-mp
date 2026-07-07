//! Decode & parse Epson iProjection "Quick Connect" QR codes.
//!
//! The QR payload is **not** the standard `WIFI:` schema — it is an Epson-specific
//! binary record, lightly obfuscated by XOR-ing every byte with `0xE5`. Decode it
//! with [`quircs`] (rqrr returns `EncodingError` on this byte mode); once
//! de-obfuscated it is a short record: a length byte, an IPv4 address, the MAC,
//! then length-prefixed ASCII fields (the credential and the SSID).
//!
//! The exact header layout varies slightly between models (e.g. wired vs
//! wireless-only projectors differ by a byte), so we do **not** rely on fixed
//! offsets for the strings. Instead we scan for length-prefixed printable-ASCII
//! fields and classify them:
//!   * a 12-hex-digit field is the Wi-Fi passphrase (the MAC in hex), and
//!   * a field containing `-` is the SSID.
//!
//! Verified against two real projectors:
//!   * RESEARCHLAB  → pw `A4D73CCDAF45`, ssid `RESEARCHLAB-fE8DSypQz51AR2Q`
//!   * EBC0E9E5     → pw `381A52C0E9E5`, ssid `EBC0E9E5-EE81fImEdb09OeF`

use std::net::Ipv4Addr;

/// Epson's fixed obfuscation key for Quick Connect QR payloads.
const XOR_KEY: u8 = 0xE5;
/// The Direct-mode address Epson projectors use when the record's IP is absent
/// or implausible.
const QUICK_CONNECT_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 88, 1);

/// The structured contents of an Epson Quick Connect QR code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpsonQr {
    /// Projector Direct-mode (Quick Connect) IPv4 address.
    pub ip: Ipv4Addr,
    /// All length-prefixed ASCII fields found in the record, in order.
    pub fields: Vec<String>,
}

impl EpsonQr {
    /// The Wi-Fi passphrase: the 12-hex-digit field, returned **verbatim**
    /// (WPA passphrases are case-sensitive). On observed projectors this equals
    /// the MAC in uppercase hex; confirmed against the OS Wi-Fi settings.
    pub fn wifi_password(&self) -> Option<&str> {
        self.fields.iter().map(|s| s.as_str()).find(|s| is_mac_hex(s))
    }

    /// The full network SSID: the field containing a `-` separator (Epson SSIDs
    /// are `<name>-<suffix>`). The projector's on-screen SSID line is often
    /// truncated; this is the untruncated value.
    pub fn ssid(&self) -> Option<&str> {
        self.fields.iter().map(|s| s.as_str()).find(|s| s.contains('-'))
    }

    /// MAC as lowercase hex with no separators — the **EasyMP auth token** for
    /// the streaming handshake (hex-decoded there, so case is irrelevant).
    pub fn mac_hex(&self) -> Option<String> {
        self.wifi_password().map(|p| p.to_ascii_lowercase())
    }

    /// The MAC as 6 raw bytes, derived from the hex passphrase.
    pub fn mac_bytes(&self) -> Option<[u8; 6]> {
        let p = self.wifi_password()?;
        let mut m = [0u8; 6];
        for (i, byte) in m.iter_mut().enumerate() {
            *byte = u8::from_str_radix(p.get(i * 2..i * 2 + 2)?, 16).ok()?;
        }
        Some(m)
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
    if d.len() < 8 {
        return None;
    }
    // IPv4 sits at offset 3 in every observed record. Validate the first octet;
    // fall back to the Quick Connect default if it looks wrong.
    let ip = if (1..=223).contains(&d[3]) {
        Ipv4Addr::new(d[3], d[4], d[5], d[6])
    } else {
        QUICK_CONNECT_IP
    };

    let fields = extract_ascii_fields(d);

    // An Epson record always carries an SSID (which contains a '-'); if we found
    // none, this is not an Epson Quick Connect QR.
    if !fields.iter().any(|f| f.contains('-')) {
        return None;
    }

    Some(EpsonQr { ip, fields })
}

/// Decode a QR from encoded image bytes (PNG/JPEG/etc.) — e.g. an uploaded photo
/// of the projector's QR screen — and parse it as an Epson record.
pub fn parse_from_image_bytes(bytes: &[u8]) -> Option<EpsonQr> {
    let luma = image::load_from_memory(bytes).ok()?.to_luma8();
    parse_from_luma(luma.width() as usize, luma.height() as usize, &luma)
}

/// Decode a QR from a raw interleaved RGB buffer (e.g. a live camera frame).
pub fn parse_from_rgb(width: u32, height: u32, rgb: &[u8]) -> Option<EpsonQr> {
    let n = (width as usize).checked_mul(height as usize)?;
    if rgb.len() < n * 3 {
        return None;
    }
    // Rec. 601 luma; QR decoders only need luminance.
    let mut luma = vec![0u8; n];
    for (i, px) in luma.iter_mut().enumerate() {
        let r = rgb[i * 3] as u32;
        let g = rgb[i * 3 + 1] as u32;
        let b = rgb[i * 3 + 2] as u32;
        *px = ((r * 299 + g * 587 + b * 114) / 1000) as u8;
    }
    parse_from_luma(width as usize, height as usize, &luma)
}

/// Decode a QR from an 8-bit grayscale buffer and parse it as an Epson record.
///
/// Real-world captures (webcam/phone photos of a projector screen) have glare,
/// low contrast, and color casts that quircs' internal binarization can't cope
/// with. So we try the raw image first (fast path for sharp, clean images), then
/// fall back to **adaptive local thresholding** at several window sizes — the
/// standard technique for decoding codes under uneven lighting.
pub fn parse_from_luma(width: usize, height: usize, gray: &[u8]) -> Option<EpsonQr> {
    if width == 0 || height == 0 || gray.len() < width * height {
        return None;
    }
    if let Some(qr) = decode_luma(width, height, gray) {
        return Some(qr);
    }
    let min_dim = width.min(height);
    for factor in [24usize, 16, 10, 6] {
        let win = ((min_dim / factor).max(15)) | 1; // odd, >= 15
        let binarized = adaptive_threshold(gray, width, height, win, 8);
        if let Some(qr) = decode_luma(width, height, &binarized) {
            return Some(qr);
        }
    }
    None
}

/// Run quircs on a luma buffer and parse the first Epson QR found.
fn decode_luma(width: usize, height: usize, gray: &[u8]) -> Option<EpsonQr> {
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

/// Adaptive (local-mean) threshold via an integral image: each pixel is compared
/// to the mean of its `win`x`win` neighborhood minus `c`. O(n), handles glare and
/// brightness gradients that global thresholds cannot.
fn adaptive_threshold(gray: &[u8], w: usize, h: usize, win: usize, c: i32) -> Vec<u8> {
    let iw = w + 1;
    let mut integral = vec![0u64; iw * (h + 1)];
    for y in 0..h {
        let mut row_sum = 0u64;
        for x in 0..w {
            row_sum += gray[y * w + x] as u64;
            integral[(y + 1) * iw + (x + 1)] = integral[y * iw + (x + 1)] + row_sum;
        }
    }
    let r = (win / 2) as i32;
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let x0 = (x as i32 - r).max(0) as usize;
            let y0 = (y as i32 - r).max(0) as usize;
            let x1 = (x as i32 + r).min(w as i32 - 1) as usize;
            let y1 = (y as i32 + r).min(h as i32 - 1) as usize;
            let area = ((x1 - x0 + 1) * (y1 - y0 + 1)) as u64;
            let sum = integral[(y1 + 1) * iw + (x1 + 1)] + integral[y0 * iw + x0]
                - integral[y0 * iw + (x1 + 1)]
                - integral[(y1 + 1) * iw + x0];
            let mean = (sum / area) as i32;
            out[y * w + x] = if gray[y * w + x] as i32 > mean - c { 255 } else { 0 };
        }
    }
    out
}

/// Scan for length-prefixed printable-ASCII fields: a byte `n` (4..=40) followed
/// by exactly `n` printable bytes. Binary header regions (IP/MAC) contain
/// non-printable bytes and are skipped, leaving just the real string fields.
fn extract_ascii_fields(d: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < d.len() {
        let len = d[pos] as usize;
        if (4..=40).contains(&len)
            && pos + 1 + len <= d.len()
            && d[pos + 1..pos + 1 + len].iter().all(|&b| (0x20..=0x7e).contains(&b))
        {
            out.push(String::from_utf8_lossy(&d[pos + 1..pos + 1 + len]).into_owned());
            pos += 1 + len;
        } else {
            pos += 1;
        }
    }
    out
}

/// True for a 12-hex-digit string (a MAC without separators).
fn is_mac_hex(s: &str) -> bool {
    s.len() == 12 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
