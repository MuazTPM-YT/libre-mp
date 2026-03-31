use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use scrap::{Capturer, Display};

mod hex;
mod wifi;
mod template;
mod capture;
mod protocol;

use std::collections::HashMap;
use std::time::{Duration, Instant};

pub const STREAM_W: u32 = 1024;
pub const STREAM_H: u32 = 768;
pub const JPEG_QUALITY: i32 = 95;
const TARGET_FPS: u64 = 24;

fn main() {
    eprintln!("=== Epson EasyMP Rust Streamer ===\n");

    // Parse CLI args: --skip-wifi --ssid <SSID> --password <PWD> --os <1-4>
    let args: Vec<String> = std::env::args().collect();
    let has_flag = |f: &str| args.iter().any(|a| a == f);
    let get_arg = |f: &str| -> Option<String> {
        args.iter()
            .position(|a| a == f)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };

    let skip_wifi = has_flag("--skip-wifi");
    let cli_ssid = get_arg("--ssid");
    let cli_password = get_arg("--password");
    let cli_os: Option<u8> = get_arg("--os").and_then(|v| v.parse().ok());

    // Determine credentials: CLI args or interactive
    let (orig_uuid, ssid, password) = if skip_wifi {
        let ssid = cli_ssid.unwrap_or_default();
        let password = cli_password.unwrap_or_default();
        eprintln!("[*] CLI mode: skip-wifi, ssid={}, os={}", ssid, cli_os.unwrap_or(3));
        (None, ssid, password)
    } else {
        let (uuid, ssid, _bssid, password) = wifi::wifi_connect();
        (uuid, ssid, password)
    };

    let os_mode = if let Some(os) = cli_os {
        os
    } else {
        eprintln!("\n[*] Select your Operating System / Display Environment:");
        eprintln!("    [1] Windows (Native DXGI)");
        eprintln!("    [2] MacOS (Native CoreGraphics)");
        eprintln!("    [3] Ubuntu / X11 (Native XShm)");
        eprintln!("    [4] Arch Linux / Wayland (grim wlroots)");
        eprint!("    Selection [1-4] (default 3): ");
        io::stderr().flush().ok();
        let mut os_sel = String::new();
        io::stdin().read_line(&mut os_sel).unwrap_or(0);
        os_sel.trim().parse::<u8>().unwrap_or(3)
    };

    // Ctrl+C: set flag immediately (cross-platform via ctrlc crate)
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl+C handler");

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

    // Auto-reconnect loop — runs until Ctrl+C
    while running.load(Ordering::Relaxed) {
        let mut client = match protocol::EpsonClient::connect(&password, &ssid) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[-] Connection failed: {e}");
                if !running.load(Ordering::Relaxed) {
                    break;
                }
                eprintln!("[*] Retrying in 3s...");
                std::thread::sleep(Duration::from_secs(3));
                continue;
            }
        };

        let mut opt_capturer = if os_mode == 2 || os_mode == 3 {
            let d = Display::primary().expect("No primary display found. Make sure you have a graphical session.");
            Some(Capturer::new(d).expect("Couldn't begin screen capture."))
        } else {
            None
        };

        let reason = stream_loop(&mut client, &mut tpl, &mut opt_capturer, &running, os_mode);
        if !running.load(Ordering::Relaxed) {
            eprintln!("\n[*] Ctrl+C received, shutting down...");
            break;
        }
        eprintln!("[-] Stream ended: {reason}");
        eprintln!("[*] Auto-reconnecting in 2s...\n");
        std::thread::sleep(Duration::from_secs(2));
    }

    // Clean shutdown
    wifi::wifi_restore(orig_uuid);
}

fn stream_loop(
    client: &mut protocol::EpsonClient,
    tpl: &mut template::Template,
    opt_capturer: &mut Option<Capturer>,
    running: &AtomicBool,
    os_mode: u8,
) -> String {
    let mut frame_idx = 0u64;
    let mut jpeg_cache: HashMap<(u16, u16, u16, u16), Vec<u8>> = HashMap::new();
    let mut last_keepalive = Instant::now();
    let mut last_auth_heartbeat = Instant::now();
    let frame_budget = Duration::from_micros(1_000_000 / TARGET_FPS);
    let my_ip = client.my_ip;

    while running.load(Ordering::Relaxed) {
        let t0 = Instant::now();

        // 1. Capture (0-copy direct from GPU/OS, or grim)
        let screen = if os_mode == 1 {
            match capture::capture_windows() {
                Some(s) => s,
                None => {
                    std::thread::sleep(Duration::from_millis(16));
                    continue;
                }
            }
        } else if os_mode == 4 {
            match capture::capture_wayland() {
                Some(s) => s,
                None => {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
            }
        } else if let Some(capturer) = opt_capturer.as_mut() {
            let w = capturer.width() as u32;
            let h = capturer.height() as u32;
            loop {
                match capturer.frame() {
                    Ok(frame) => {
                        break capture::resize_bgra_to_rgb(
                            &frame,
                            w,
                            h,
                            STREAM_W,
                            STREAM_H,
                        );
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if !running.load(Ordering::Relaxed) {
                            return "Ctrl+C".to_string();
                        }
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    Err(e) => return format!("Capture Error: {e}"),
                }
            }
        } else {
            std::thread::sleep(Duration::from_millis(50));
            continue;
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

        // 3. Send frame
        if let Err(e) = protocol::send_frame(&mut client.s_video, &tpl.buf[0..tpl.first_frame_end])
        {
            return format!("{e}");
        }
        let t_send = t0.elapsed();

        // 4. Drain auth channel
        protocol::drain_auth(&mut client.s_auth, my_ip);

        // 5. Proactive auth heartbeat every 30s
        if last_auth_heartbeat.elapsed() > Duration::from_secs(30) {
            let _ = client.s_auth.write_all(&protocol::response_0x0108(my_ip));
            last_auth_heartbeat = Instant::now();
        }

        // 6. Aux keepalive every 3s
        if last_keepalive.elapsed() > Duration::from_secs(3) {
            if let Err(e) = protocol::send_keepalive(&mut client.s_aux) {
                return format!("Keepalive: {e}");
            }
            last_keepalive = Instant::now();
        }

        // 7. Frame rate limiter
        let elapsed = t0.elapsed();
        if elapsed < frame_budget {
            std::thread::sleep(frame_budget - elapsed);
        }

        frame_idx += 1;
        if frame_idx <= 5 || frame_idx % 100 == 0 {
            let total_time_ms = t0.elapsed().as_millis().max(1) as f64;
            let fps = 1000.0 / total_time_ms;
            
            // Wayland black screen detector! 
            // If the buffer is completely pitch black (0 bytes), it guarantees standard OS security is blocking it!
            let non_zero_pixels = screen.iter().take(10000).filter(|&&b| b > 0).count();

            eprintln!(
                "  Frame {}: cap={:.0}ms enc={:.0}ms send={:.0}ms total={:.0}ms ({:.1}fps)",
                frame_idx,
                t_capture.as_millis(),
                (t_encode - t_capture).as_millis(),
                (t_send - t_encode).as_millis(),
                total_time_ms,
                fps,
            );

            if non_zero_pixels == 0 && os_mode == 3 {
                eprintln!("\n[!] CRITICAL WARNING: Entire screen capture is completely BLACK.");
                eprintln!("    -> Are you running Ubuntu on Wayland? Traditional GPU grabbers cannot see Wayland windows!");
                eprintln!("    -> SOLUTION: Restart the streamer and select Option [4], OR log out and use 'Ubuntu on Xorg'!\n");
            }
        }
    }

    "Ctrl+C".to_string()
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
