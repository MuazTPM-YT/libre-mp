use std::process::Command;
use turbojpeg::{Compressor, Image, PixelFormat, Subsamp};

use crate::{STREAM_W, STREAM_H, JPEG_QUALITY};

/// Capture screen via grim PPM, resize to STREAM_W × STREAM_H.
pub fn capture_screen() -> Option<Vec<u8>> {
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

/// Extract RGB tile pixels from full screen buffer.
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
/// This prevents pad_jpeg from truncating the JPEG (which causes gray blocks).
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

    // Try starting quality, reduce until JPEG fits
    let mut quality = JPEG_QUALITY;
    loop {
        let mut comp = Compressor::new().expect("turbojpeg");
        let _ = comp.set_quality(quality);
        let _ = comp.set_subsamp(Subsamp::Sub2x2); // 4:2:0 required

        let jpeg = comp.compress_to_vec(image.clone()).unwrap_or_default();

        if jpeg.len() <= max_size || quality <= 5 {
            return jpeg;
        }

        // Reduce quality by 5 and retry (finer steps = better slot utilization)
        quality -= 5;
        if quality < 5 {
            quality = 5;
        }
    }
}
