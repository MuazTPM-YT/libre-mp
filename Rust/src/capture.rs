use std::process::Command;
use turbojpeg::{Compressor, Image, PixelFormat, Subsamp};

use crate::{STREAM_W, STREAM_H, JPEG_QUALITY};

// ─── Platform-specific screen capture ────────────────────────────────────────

/// Capture screen, return RGB pixels resized to STREAM_W × STREAM_H.
/// Auto-detects Wayland vs X11 and uses the appropriate tool.
#[cfg(target_os = "linux")]
pub fn capture_screen() -> Option<Vec<u8>> {
    // Detect display server
    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|v| v == "wayland")
            .unwrap_or(false);

    if is_wayland {
        capture_wayland()
    } else {
        capture_x11()
    }
}

/// Wayland: use grim (pipe PPM to stdout, zero-copy)
#[cfg(target_os = "linux")]
fn capture_wayland() -> Option<Vec<u8>> {
    let output = Command::new("grim")
        .args(["-c", "-t", "ppm", "-"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let (w, h, data_start) = parse_ppm_header(&output.stdout)?;
    let expected = (w as usize) * (h as usize) * 3;
    let rgb_data = &output.stdout[data_start..];

    if rgb_data.len() < expected {
        return None;
    }

    Some(resize_nearest(rgb_data, w, h, STREAM_W, STREAM_H))
}

/// X11: try gnome-screenshot, scrot, then import (ImageMagick)
#[cfg(target_os = "linux")]
fn capture_x11() -> Option<Vec<u8>> {
    let tmp = "/tmp/epson_cap.ppm";

    // Try 1: gnome-screenshot (GNOME/Ubuntu)
    let ok = Command::new("gnome-screenshot")
        .args(["-f", "/tmp/epson_cap.png"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if ok {
        // Convert PNG → PPM via ffmpeg (universally available)
        let output = Command::new("ffmpeg")
            .args(["-y", "-i", "/tmp/epson_cap.png", "-f", "image2", "-pix_fmt", "rgb24", tmp])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        if output.map(|s| s.success()).unwrap_or(false) {
            let data = std::fs::read(tmp).ok()?;
            let (w, h, data_start) = parse_ppm_header(&data)?;
            let expected = (w as usize) * (h as usize) * 3;
            let rgb_data = &data[data_start..];
            if rgb_data.len() >= expected {
                return Some(resize_nearest(rgb_data, w, h, STREAM_W, STREAM_H));
            }
        }
    }

    // Try 2: import (ImageMagick) — pipes PPM directly
    let output = Command::new("import")
        .args(["-window", "root", "ppm:-"])
        .output()
        .ok()?;

    if output.status.success() && !output.stdout.is_empty() {
        let (w, h, data_start) = parse_ppm_header(&output.stdout)?;
        let expected = (w as usize) * (h as usize) * 3;
        let rgb_data = &output.stdout[data_start..];
        if rgb_data.len() >= expected {
            return Some(resize_nearest(rgb_data, w, h, STREAM_W, STREAM_H));
        }
    }

    // Try 3: scrot → temp file → read PPM
    let ok = Command::new("scrot")
        .args(["/tmp/epson_cap.png"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if ok {
        let output = Command::new("ffmpeg")
            .args(["-y", "-i", "/tmp/epson_cap.png", "-f", "image2", "-pix_fmt", "rgb24", tmp])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        if output.map(|s| s.success()).unwrap_or(false) {
            let data = std::fs::read(tmp).ok()?;
            let (w, h, data_start) = parse_ppm_header(&data)?;
            let expected = (w as usize) * (h as usize) * 3;
            let rgb_data = &data[data_start..];
            if rgb_data.len() >= expected {
                return Some(resize_nearest(rgb_data, w, h, STREAM_W, STREAM_H));
            }
        }
    }

    None
}

/// macOS: capture via screencapture CLI → temp BMP → decode.
#[cfg(target_os = "macos")]
pub fn capture_screen() -> Option<Vec<u8>> {
    let tmp = "/tmp/epson_cap.bmp";

    let status = Command::new("screencapture")
        .args(["-x", "-C", "-t", "bmp", tmp])
        .status()
        .ok()?;

    if !status.success() {
        return None;
    }

    let data = std::fs::read(tmp).ok()?;
    let (w, h, rgb) = decode_bmp(&data)?;
    Some(resize_nearest(&rgb, w, h, STREAM_W, STREAM_H))
}

/// Windows: capture via PowerShell System.Drawing → temp BMP → decode.
#[cfg(target_os = "windows")]
pub fn capture_screen() -> Option<Vec<u8>> {
    let tmp = std::env::temp_dir().join("epson_cap.bmp");
    let tmp_str = tmp.to_str()?;

    let script = format!(
        r#"Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; $s=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds; $b=New-Object System.Drawing.Bitmap($s.Width,$s.Height); $g=[System.Drawing.Graphics]::FromImage($b); $g.CopyFromScreen($s.Location,[System.Drawing.Point]::Empty,$s.Size); $b.Save('{}'); $g.Dispose(); $b.Dispose()"#,
        tmp_str
    );

    let status = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .status()
        .ok()?;

    if !status.success() {
        return None;
    }

    let data = std::fs::read(&tmp).ok()?;
    let (w, h, rgb) = decode_bmp(&data)?;
    Some(resize_nearest(&rgb, w, h, STREAM_W, STREAM_H))
}

// ─── BMP decoder (for macOS/Windows) ─────────────────────────────────────────

/// Minimal BMP decoder — extracts RGB pixels from uncompressed 24/32-bit BMP.
#[cfg(any(target_os = "macos", target_os = "windows"))]
fn decode_bmp(data: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    if data.len() < 54 || &data[0..2] != b"BM" {
        return None;
    }

    let data_offset = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;
    let w = i32::from_le_bytes([data[18], data[19], data[20], data[21]]) as u32;
    let h_signed = i32::from_le_bytes([data[22], data[23], data[24], data[25]]);
    let bpp = u16::from_le_bytes([data[28], data[29]]) as usize;

    let h = h_signed.unsigned_abs();
    let top_down = h_signed < 0;
    let bytes_per_pixel = bpp / 8;

    if bytes_per_pixel < 3 || data_offset >= data.len() {
        return None;
    }

    let row_size = ((bpp as u32 * w + 31) / 32 * 4) as usize;
    let mut rgb = vec![0u8; (w * h * 3) as usize];

    for y in 0..h as usize {
        let src_y = if top_down { y } else { (h as usize) - 1 - y };
        let row_start = data_offset + src_y * row_size;

        for x in 0..w as usize {
            let si = row_start + x * bytes_per_pixel;
            let di = (y * w as usize + x) * 3;
            if si + 2 < data.len() {
                // BMP stores BGR
                rgb[di] = data[si + 2];     // R
                rgb[di + 1] = data[si + 1]; // G
                rgb[di + 2] = data[si];     // B
            }
        }
    }

    Some((w, h, rgb))
}

// ─── PPM decoder (for Linux grim) ────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn parse_ppm_header(data: &[u8]) -> Option<(u32, u32, usize)> {
    if data.len() < 7 || data[0] != b'P' || data[1] != b'6' {
        return None;
    }
    let mut pos = 3;
    while pos < data.len() && data[pos] == b'#' {
        while pos < data.len() && data[pos] != b'\n' { pos += 1; }
        pos += 1;
    }
    let w_start = pos;
    while pos < data.len() && data[pos].is_ascii_digit() { pos += 1; }
    let w: u32 = std::str::from_utf8(&data[w_start..pos]).ok()?.parse().ok()?;
    pos += 1;
    let h_start = pos;
    while pos < data.len() && data[pos].is_ascii_digit() { pos += 1; }
    let h: u32 = std::str::from_utf8(&data[h_start..pos]).ok()?.parse().ok()?;
    pos += 1;
    while pos < data.len() && data[pos].is_ascii_digit() { pos += 1; }
    pos += 1;
    Some((w, h, pos))
}

// ─── Common: resize + adaptive encode ────────────────────────────────────────

fn resize_nearest(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut dst = vec![0u8; (dw * dh * 3) as usize];
    let sw_usize = sw as usize;
    for y in 0..dh {
        let sy = ((y as u64 * sh as u64) / dh as u64) as usize;
        let dst_row = (y as usize) * (dw as usize) * 3;
        let src_row = sy * sw_usize * 3;
        for x in 0..dw {
            let sx = ((x as u64 * sw as u64) / dw as u64) as usize;
            let si = src_row + sx * 3;
            let di = dst_row + (x as usize) * 3;
            dst[di] = src[si];
            dst[di + 1] = src[si + 1];
            dst[di + 2] = src[si + 2];
        }
    }
    dst
}

fn extract_tile(screen: &[u8], x: u16, y: u16, w: u16, h: u16) -> Vec<u8> {
    let sw = STREAM_W;
    let sh = STREAM_H;
    let cx = (x as u32).min(sw.saturating_sub(1));
    let cy = (y as u32).min(sh.saturating_sub(1));
    let cw = (w as u32).min(sw - cx);
    let ch = (h as u32).min(sh - cy);

    let mut rgb_buf = vec![0u8; (cw * ch * 3) as usize];
    let mut idx = 0;
    for row in cy..cy + ch {
        let src_row = (row as usize) * (sw as usize) * 3;
        for col in cx..cx + cw {
            let si = src_row + (col as usize) * 3;
            rgb_buf[idx] = screen[si];
            rgb_buf[idx + 1] = screen[si + 1];
            rgb_buf[idx + 2] = screen[si + 2];
            idx += 3;
        }
    }
    rgb_buf
}

/// Encode a tile, adaptively reducing quality until JPEG fits in max_size.
pub fn encode_tile_adaptive(
    screen: &[u8],
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    max_size: usize,
) -> Vec<u8> {
    let cw = (w as u32).min(STREAM_W - (x as u32).min(STREAM_W.saturating_sub(1)));
    let ch = (h as u32).min(STREAM_H - (y as u32).min(STREAM_H.saturating_sub(1)));

    let rgb_buf = extract_tile(screen, x, y, w, h);

    let image = Image {
        pixels: rgb_buf.as_slice(),
        width: cw as usize,
        pitch: (cw * 3) as usize,
        height: ch as usize,
        format: PixelFormat::RGB,
    };

    let mut quality = JPEG_QUALITY;
    loop {
        let mut comp = Compressor::new().expect("turbojpeg");
        let _ = comp.set_quality(quality);
        let _ = comp.set_subsamp(Subsamp::Sub2x2); // 4:2:0 required

        let jpeg = comp.compress_to_vec(image.clone()).unwrap_or_default();

        if jpeg.len() <= max_size || quality <= 5 {
            return jpeg;
        }

        quality -= 5;
        if quality < 5 {
            quality = 5;
        }
    }
}
