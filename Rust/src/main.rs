mod hex;
mod wifi;
mod template;
mod capture;
mod protocol;

use std::collections::HashMap;
use std::time::{Duration, Instant};

pub const STREAM_W: u32 = 1024;
pub const STREAM_H: u32 = 768;
pub const JPEG_QUALITY: i32 = 50;

fn main() {
    eprintln!("=== Epson EasyMP Rust Streamer ===\n");

    // 1. Auto-connect Wi-Fi
    let orig_uuid = wifi::wifi_connect();

    // Setup Ctrl+C handler for clean Wi-Fi restoration
    let orig_uuid_clone = orig_uuid.clone();
    ctrlc_handler(orig_uuid_clone);

    // 2. Connect to projector
    let mut client = match protocol::EpsonClient::connect() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[-] Connection failed: {e}");
            wifi::wifi_restore(orig_uuid);
            std::process::exit(1);
        }
    };

    // 3. Load template
    let template_path = std::env::current_dir()
        .unwrap()
        .parent()
        .map(|p| p.join("windows_perfect_stream.bin"))
        .unwrap_or_else(|| std::path::PathBuf::from("../windows_perfect_stream.bin"));

    // Try multiple locations for the template file
    let template_path = if template_path.exists() {
        template_path
    } else if std::path::Path::new("windows_perfect_stream.bin").exists() {
        std::path::PathBuf::from("windows_perfect_stream.bin")
    } else if std::path::Path::new("../windows_perfect_stream.bin").exists() {
        std::path::PathBuf::from("../windows_perfect_stream.bin")
    } else {
        eprintln!("[-] Cannot find windows_perfect_stream.bin!");
        wifi::wifi_restore(orig_uuid);
        std::process::exit(1);
    };

    let mut tpl = match template::Template::load(template_path.to_str().unwrap()) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[-] Template load failed: {e}");
            wifi::wifi_restore(orig_uuid);
            std::process::exit(1);
        }
    };

    // 4. Initialize turbojpeg compressor
    let mut compressor = capture::new_compressor();

    eprintln!(
        "\n[*] Starting live stream — {} slots, {}×{} viewport",
        tpl.slots.len(),
        STREAM_W,
        STREAM_H
    );

    // 5. Streaming loop
    let mut frame_idx = 0u64;
    let mut jpeg_cache: HashMap<(u16, u16, u16, u16), Vec<u8>> = HashMap::new();

    loop {
        let t0 = Instant::now();

        // Capture screen
        let screen = match capture::capture_screen() {
            Some(s) => s,
            None => {
                eprintln!("[-] Capture failed, retrying...");
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
        };
        let t_capture = t0.elapsed();

        // Encode and swap all tiles
        jpeg_cache.clear();
        for idx in 0..tpl.slots.len() {
            let slot = tpl.slots[idx].clone();
            let key = (slot.x, slot.y, slot.w, slot.h);

            let jpeg_raw = jpeg_cache
                .entry(key)
                .or_insert_with(|| capture::encode_tile(&mut compressor, &screen, slot.x, slot.y, slot.w, slot.h))
                .clone();

            let padded = template::pad_jpeg(&jpeg_raw, slot.size);
            tpl.swap(idx, &padded);
        }
        let t_encode = t0.elapsed();

        // Send template through TCP chunker
        match protocol::send_chunked(&mut client.s_video, &tpl.buf) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("[-] Send error: {e}");
                break;
            }
        }
        let t_total = t0.elapsed();

        frame_idx += 1;
        if frame_idx <= 3 || frame_idx % 10 == 0 {
            eprintln!(
                "  Frame {}: capture={:.0}ms encode={:.0}ms send={:.0}ms total={:.0}ms",
                frame_idx,
                t_capture.as_millis(),
                (t_encode - t_capture).as_millis(),
                (t_total - t_encode).as_millis(),
                t_total.as_millis(),
            );
        }
    }

    // Cleanup
    wifi::wifi_restore(orig_uuid);
}

fn ctrlc_handler(orig_uuid: Option<String>) {
    // Use a simple approach: register a handler that restores Wi-Fi on ctrl+c
    let _ = std::thread::spawn(move || {
        // Wait for ctrl+c signal
        let mut sigs =
            signal_hook::iterator::Signals::new(&[signal_hook::consts::SIGINT]).unwrap();
        for _ in sigs.forever() {
            wifi::wifi_restore(orig_uuid);
            std::process::exit(0);
        }
    });
}
