use std::process::Command;
use turbojpeg::{Compressor, Image, PixelFormat, Subsamp};

use crate::{STREAM_W, STREAM_H, JPEG_QUALITY};

// ─── Wayland (grim) Capture ───────────────────────────────────────────────

/// Captures the screen on Wayland using the `grim` utility.
pub fn capture_wayland() -> Option<Vec<u8>> {
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

/// Parses the PPM image header to extract width, height, and pixel data offset.
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

/// Resizes an RGB image using fast nearest-neighbor interpolation.
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

// ─── High-Performance BGRA Resizer ─────────────────────────────────────────

/// Resizes a BGRA image and converts it to RGB simultaneously.
pub fn resize_bgra_to_rgb(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut dst = vec![0u8; (dw * dh * 3) as usize];
    let sw_usize = sw as usize;
    for y in 0..dh {
        let sy = ((y as u64 * sh as u64) / dh as u64) as usize;
        let dst_row = (y as usize) * (dw as usize) * 3;
        let src_row = sy * sw_usize * 4; // 4 bytes per pixel for BGRA
        for x in 0..dw {
            let sx = ((x as u64 * sw as u64) / dw as u64) as usize;
            let si = src_row + sx * 4;
            let di = dst_row + (x as usize) * 3;
            // BGRA -> RGB
            dst[di]     = src[si + 2]; // R
            dst[di + 1] = src[si + 1]; // G
            dst[di + 2] = src[si];     // B
        }
    }
    dst
}

/// Extracts a bounded rectangular tile from the main RGB screen buffer.
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

// ─── Windows GDI Capture (with cursor) ────────────────────────────────────

#[cfg(windows)]
/// Captures the screen on Windows using GDI, including the mouse cursor.
pub fn capture_windows() -> Option<Vec<u8>> {
    use std::ptr::null_mut;
    use winapi::um::wingdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDeviceCaps,
        GetDIBits, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
    };
    use winapi::um::winuser::{
        DrawIconEx, GetCursorInfo, GetDC, GetIconInfo, ReleaseDC, CURSORINFO, CURSOR_SHOWING,
        ICONINFO,
    };
    use winapi::shared::minwindef::TRUE;
    
    unsafe {
        let hdc_screen = GetDC(null_mut());
        if hdc_screen.is_null() {
            return None;
        }

        // 118 = DESKTOPHORZRES, 117 = DESKTOPVERTRES
        let width = GetDeviceCaps(hdc_screen, 118);
        let height = GetDeviceCaps(hdc_screen, 117);
        
        let width = if width == 0 { GetDeviceCaps(hdc_screen, 8) } else { width }; // fallback to HORZRES
        let height = if height == 0 { GetDeviceCaps(hdc_screen, 10) } else { height }; // fallback to VERTRES

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbm_screen = CreateCompatibleBitmap(hdc_screen, width, height);

        let hbm_old = SelectObject(hdc_mem, hbm_screen as *mut _);

        // Copy screen
        BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY);

        // Draw cursor
        let mut ci: CURSORINFO = std::mem::zeroed();
        ci.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
        if GetCursorInfo(&mut ci) == TRUE {
            if ci.flags == CURSOR_SHOWING {
                let mut ii: ICONINFO = std::mem::zeroed();
                if GetIconInfo(ci.hCursor, &mut ii) == TRUE {
                    // Offset by hotspot
                    let draw_x = ci.ptScreenPos.x - ii.xHotspot as i32;
                    let draw_y = ci.ptScreenPos.y - ii.yHotspot as i32;
                    DrawIconEx(
                        hdc_mem,
                        draw_x,
                        draw_y,
                        ci.hCursor,
                        0,
                        0,
                        0,
                        null_mut(),
                        3, // DI_NORMAL
                    );
                    
                    if !ii.hbmColor.is_null() { DeleteObject(ii.hbmColor as *mut _); }
                    if !ii.hbmMask.is_null() { DeleteObject(ii.hbmMask as *mut _); }
                }
            }
        }

        // Extract DIB bits
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width;
        bmi.bmiHeader.biHeight = -height; // Top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut bgra_buf = vec![0u8; (width * height * 4) as usize];
        let res = GetDIBits(
            hdc_screen,
            hbm_screen,
            0,
            height as u32,
            bgra_buf.as_mut_ptr() as *mut _,
            &mut bmi,
            DIB_RGB_COLORS,
        );

        SelectObject(hdc_mem, hbm_old);
        DeleteObject(hbm_screen as *mut _);
        DeleteDC(hdc_mem);
        ReleaseDC(null_mut(), hdc_screen);

        if res == 0 {
            return None;
        }

        Some(crate::capture::resize_bgra_to_rgb(
            &bgra_buf,
            width as u32,
            height as u32,
            crate::STREAM_W,
            crate::STREAM_H,
        ))
    }
}
#[cfg(not(windows))]
/// Dummy implementation of Windows screen capture for non-Windows platforms.
pub fn capture_windows() -> Option<Vec<u8>> {
    None
}

