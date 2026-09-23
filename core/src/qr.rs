//! epson quick connect qr: xor 0xE5 binary record, not WIFI: schema. fields found by scan, not offsets

use std::net::Ipv4Addr;

// epson xor key
const XOR_KEY: u8 = 0xE5;
// quick connect ip when record ip look wrong
const QUICK_CONNECT_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 88, 1);

// what epson qr hold
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpsonQr {
    // projector ip
    pub ip: Ipv4Addr,
    // all length-prefixed ascii fields, in order
    pub fields: Vec<String>,
}

impl EpsonQr {
    // wi-fi password: 12-hex-digit field, as is (case matters)
    pub fn wifi_password(&self) -> Option<&str> {
        self.fields.iter().map(|s| s.as_str()).find(|s| is_mac_hex(s))
    }

    // full ssid: field with '-', else first non-password field
    pub fn ssid(&self) -> Option<&str> {
        let mut fields = self.fields.iter().map(|s| s.as_str());
        fields.clone().find(|s| s.contains('-')).or_else(|| fields.find(|s| !is_mac_hex(s)))
    }

    // mac lowercase hex = easymp auth token
    pub fn mac_hex(&self) -> Option<String> {
        self.wifi_password().map(|p| p.to_ascii_lowercase())
    }

    // mac as 6 bytes
    pub fn mac_bytes(&self) -> Option<[u8; 6]> {
        let p = self.wifi_password()?;
        let mut m = [0u8; 6];
        for (i, byte) in m.iter_mut().enumerate() {
            *byte = u8::from_str_radix(p.get(i * 2..i * 2 + 2)?, 16).ok()?;
        }
        Some(m)
    }
}

// undo epson xor
pub fn deobfuscate(payload: &[u8]) -> Vec<u8> {
    payload.iter().map(|b| b ^ XOR_KEY).collect()
}

// parse raw (still xor'd) payload
pub fn parse(raw_payload: &[u8]) -> Option<EpsonQr> {
    parse_deobfuscated(&deobfuscate(raw_payload))
}

// parse un-xor'd record
pub fn parse_deobfuscated(d: &[u8]) -> Option<EpsonQr> {
    if d.len() < 8 {
        return None;
    }
    // ip at offset 3; bad first octet = quick connect default
    let ip = if (1..=223).contains(&d[3]) {
        Ipv4Addr::new(d[3], d[4], d[5], d[6])
    } else {
        QUICK_CONNECT_IP
    };

    let fields = extract_ascii_fields(d);

    // no epson ssid or password field = not epson qr
    if !fields.iter().any(|f| f.contains('-') || is_mac_hex(f)) {
        return None;
    }

    Some(EpsonQr { ip, fields })
}

// read epson qr from photo bytes (png, jpeg)
pub fn parse_from_image_bytes(bytes: &[u8]) -> Option<EpsonQr> {
    let luma = image::load_from_memory(bytes).ok()?.to_luma8();
    parse_from_luma(luma.width() as usize, luma.height() as usize, &luma)
}

// read epson qr from rgb frame (camera)
pub fn parse_from_rgb(width: u32, height: u32, rgb: &[u8]) -> Option<EpsonQr> {
    let n = (width as usize).checked_mul(height as usize)?;
    if rgb.len() < n * 3 {
        return None;
    }
    // rec. 601 luma; qr only need brightness
    let mut luma = vec![0u8; n];
    for (i, px) in luma.iter_mut().enumerate() {
        let r = rgb[i * 3] as u32;
        let g = rgb[i * 3 + 1] as u32;
        let b = rgb[i * 3 + 2] as u32;
        *px = ((r * 299 + g * 587 + b * 114) / 1000) as u8;
    }
    parse_from_luma(width as usize, height as usize, &luma)
}

// read epson qr from gray image: raw first, then adaptive threshold for glare
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

// quircs over gray image, first epson qr wins
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

// local-mean threshold via integral image, o(n), beats glare
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

// find length-prefixed printable fields (len 4..=40)
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

// 12 hex digits = mac
fn is_mac_hex(s: &str) -> bool {
    s.len() == 12 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
