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
const TARGET_FPS: u64 = 24;

fn main() {
    eprintln!("=== Epson EasyMP Rust Streamer ===\n");

    let orig_uuid = wifi::wifi_connect();
    let orig_uuid_clone = orig_uuid.clone();
    ctrlc_handler(orig_uuid_clone);

    let mut tpl = match template::Template::load(&find_template()) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[-] Template load failed: {e}");
            wifi::wifi_restore(orig_uuid);
            std::process::exit(1);
        }
    };

    eprintln!(
        "[*] Config: {} tiles/frame, {}KB/frame, target {}fps",
        tpl.first_frame_slots,
        tpl.first_frame_end / 1024,
        TARGET_FPS,
    );
    for i in 0..tpl.first_frame_slots {
        let s = &tpl.slots[i];
        eprintln!(
            "    Tile {}: ({}x{} @ {},{}) slot={}B",
            i, s.w, s.h, s.x, s.y, s.size
        );
    }

    // Auto-reconnect loop
    loop {
        let mut client = match protocol::EpsonClient::connect() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[-] Connection failed: {e}");
                eprintln!("[*] Retrying in 3s...");
                std::thread::sleep(Duration::from_secs(3));
                continue;
            }
        };

        let reason = stream_loop(&mut client, &mut tpl);
        eprintln!("[-] Stream ended: {reason}");
        eprintln!("[*] Auto-reconnecting in 2s...\n");
        std::thread::sleep(Duration::from_secs(2));
    }
}

fn stream_loop(
    client: &mut protocol::EpsonClient,
    tpl: &mut template::Template,
) -> String {
    let mut frame_idx = 0u64;
    let mut jpeg_cache: HashMap<(u16, u16, u16, u16), Vec<u8>> = HashMap::new();
    let mut last_keepalive = Instant::now();
    let frame_budget = Duration::from_micros(1_000_000 / TARGET_FPS);
    let my_ip = client.my_ip;

    loop {
        let t0 = Instant::now();

        // 1. Capture
        let screen = match capture::capture_screen() {
            Some(s) => s,
            None => {
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
        };
        let t_capture = t0.elapsed();

        // 2. Encode + swap — adaptive quality per tile
        jpeg_cache.clear();
        for idx in 0..tpl.first_frame_slots {
            let slot = tpl.slots[idx].clone();
            let key = (slot.x, slot.y, slot.w, slot.h);

            let jpeg_raw = jpeg_cache
                .entry(key)
                .or_insert_with(|| {
                    capture::encode_tile_adaptive(
                        &screen, slot.x, slot.y, slot.w, slot.h, slot.size,
                    )
                })
                .clone();

            let padded = template::pad_jpeg(&jpeg_raw, slot.size);
            tpl.swap(idx, &padded);
        }
        let t_encode = t0.elapsed();

        // 3. Send — write_all (TCP handles segmentation, frame limiter prevents buildup)
        if let Err(e) = protocol::send_frame(&mut client.s_video, &tpl.buf[0..tpl.first_frame_end])
        {
            return format!("{e}");
        }
        let t_send = t0.elapsed();

        // 4. Drain auth channel — respond to projector heartbeat queries
        //    This prevents the 50-second RST timeout!
        protocol::drain_auth(&mut client.s_auth, my_ip);

        // 5. Keepalive on aux channel every 5s
        if last_keepalive.elapsed() > Duration::from_secs(5) {
            if let Err(e) = protocol::send_keepalive(&mut client.s_aux) {
                return format!("Keepalive: {e}");
            }
            last_keepalive = Instant::now();
        }

        frame_idx += 1;
        if frame_idx <= 5 || frame_idx % 100 == 0 {
            let fps = 1000.0 / t_send.as_millis().max(1) as f64;
            eprintln!(
                "  Frame {}: cap={:.0}ms enc={:.0}ms send={:.0}ms total={:.0}ms ({:.1}fps)",
                frame_idx,
                t_capture.as_millis(),
                (t_encode - t_capture).as_millis(),
                (t_send - t_encode).as_millis(),
                t_send.as_millis(),
                fps,
            );
        }

        // 6. Frame rate limiter — sleep to match 24fps target
        let elapsed = t0.elapsed();
        if elapsed < frame_budget {
            std::thread::sleep(frame_budget - elapsed);
        }
    }
}

fn find_template() -> String {
    for path in [
        "../windows_perfect_stream.bin",
        "windows_perfect_stream.bin",
    ] {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    eprintln!("[-] Cannot find windows_perfect_stream.bin!");
    std::process::exit(1);
}

fn ctrlc_handler(orig_uuid: Option<String>) {
    let _ = std::thread::spawn(move || {
        let mut sigs =
            signal_hook::iterator::Signals::new(&[signal_hook::consts::SIGINT]).unwrap();
        for _ in sigs.forever() {
            wifi::wifi_restore(orig_uuid);
            std::process::exit(0);
        }
    });
}
