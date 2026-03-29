use std::process::Command;
use turbojpeg::{Compressor, Image, PixelFormat, Subsamp};

use crate::{STREAM_W, STREAM_H, JPEG_QUALITY};

pub fn capture_screen() -> Option<image::RgbaImage> {
    let output = Command::new("grim").args(["-t", "png", "-"]).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let img = image::load_from_memory_with_format(&output.stdout, image::ImageFormat::Png).ok()?;
    let resized = img.resize_exact(STREAM_W, STREAM_H, image::imageops::FilterType::Triangle);
    Some(resized.to_rgba8())
}

pub fn encode_tile(
    compressor: &mut Compressor,
    screen: &image::RgbaImage,
    x: u16,
    y: u16,
    w: u16,
    h: u16,
) -> Vec<u8> {
    let (sw, sh) = screen.dimensions();
    let cx = (x as u32).min(sw.saturating_sub(1));
    let cy = (y as u32).min(sh.saturating_sub(1));
    let cw = (w as u32).min(sw - cx);
    let ch = (h as u32).min(sh - cy);

    // Extract RGB pixels (strip alpha) from the crop region
    let mut rgb_buf = vec![0u8; (cw * ch * 3) as usize];
    let mut idx = 0;
    for row in cy..cy + ch {
        for col in cx..cx + cw {
            let px = screen.get_pixel(col, row);
            rgb_buf[idx] = px[0];
            rgb_buf[idx + 1] = px[1];
            rgb_buf[idx + 2] = px[2];
            idx += 3;
        }
    }

    let image = Image {
        pixels: rgb_buf.as_slice(),
        width: cw as usize,
        pitch: (cw * 3) as usize,
        height: ch as usize,
        format: PixelFormat::RGB,
    };

    compressor.compress_to_vec(image).unwrap_or_default()
}

pub fn new_compressor() -> Compressor {
    let mut compressor = Compressor::new().expect("turbojpeg init");
    let _ = compressor.set_quality(JPEG_QUALITY);
    let _ = compressor.set_subsamp(Subsamp::Sub2x2); // 4:2:0
    compressor
}
