use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use libremp_core::capture::FrameGrabber;
use libremp_core::protocol::{VideoTile, KEYFRAME_TILES};
use libremp_core::{capture, protocol, wifi};

use std::time::{Duration, Instant};

const TARGET_FPS: u64 = 24;
/// Silent-audio slice cadence on the aux channel (matches the Windows client).
const AUDIO_SLICE: Duration = Duration::from_millis(100);

/// Main entry point for the Epson EasyMP Rust Streamer.
/// Handles CLI arguments, prompts, and manages the main application lifecycle.
fn main() {
    eprintln!("=== Epson EasyMP Rust Streamer ===\n");

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
    // Optional explicit projector address; otherwise found via the default gateway(s).
    let cli_proj_ip: Option<std::net::Ipv4Addr> =
        get_arg("--projector-ip").and_then(|v| v.parse().ok());
    // The 4-digit "Projector Keyword" some projectors show on screen.
    let cli_keyword = get_arg("--keyword");
    // `--os` is still accepted for backwards compatibility but ignored: the
    // capture backend is auto-detected.

    // Determine credentials: CLI args or interactive
    let (orig_uuid, ssid, password) = if skip_wifi {
        let ssid = cli_ssid.unwrap_or_default();
        let password = cli_password.unwrap_or_default();
        eprintln!("[*] CLI mode: skip-wifi, ssid={}", ssid);
        (None, ssid, password)
    } else {
        let (uuid, ssid, _bssid, password) = wifi::wifi_connect();
        (uuid, ssid, password)
    };

    // Ctrl+C / SIGTERM: stop the loop so we can say goodbye to the projector.
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl+C handler");

    // The GUI stops us by closing our stdin (works the same on every OS).
    if has_flag("--stop-on-stdin-eof") {
        let r = running.clone();
        std::thread::spawn(move || {
            let _ = std::io::stdin().read_to_end(&mut Vec::new());
            r.store(false, Ordering::Relaxed);
        });
    }

    eprintln!("[*] Config: {} tiles/frame, target {}fps", KEYFRAME_TILES.len(), TARGET_FPS);

    // Pick the capture backend once, before connecting: on Wayland this shows the
    // share dialog, and a reconnect must not ask again.
    let mut grabber = match capture::detect_grabber() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("[-] {e}");
            wifi::wifi_restore(orig_uuid);
            std::process::exit(3);
        }
    };

    // Consecutive failed connects before exiting with status 2 (unset = retry forever).
    let give_up_after: Option<u32> = get_arg("--give-up-after").and_then(|v| v.parse().ok());
    let mut failures = 0u32;

    // Auto-reconnect loop — runs until stopped
    while running.load(Ordering::Relaxed) {
        let mut client = match protocol::EpsonClient::connect(
            &password,
            &ssid,
            cli_proj_ip,
            cli_keyword.as_deref(),
        ) {
            Ok(c) => {
                failures = 0;
                c
            }
            Err(e) => {
                eprintln!("[-] Connection failed: {e}");
                failures += 1;
                if give_up_after.is_some_and(|n| failures >= n) {
                    eprintln!("[-] Giving up after {failures} failed attempts.");
                    wifi::wifi_restore(orig_uuid);
                    std::process::exit(2);
                }
                if !running.load(Ordering::Relaxed) {
                    break;
                }
                eprintln!("[*] Retrying in 3s...");
                std::thread::sleep(Duration::from_secs(3));
                continue;
            }
        };

        let reason = stream_loop(&mut client, grabber.as_mut(), &running);
        if !running.load(Ordering::Relaxed) {
            eprintln!("\n[*] Stop requested, disconnecting...");
            client.disconnect();
            break;
        }
        eprintln!("[-] Stream ended: {reason}");
        eprintln!("[*] Auto-reconnecting in 2s...\n");
        std::thread::sleep(Duration::from_secs(2));
    }

    // Clean shutdown
    wifi::wifi_restore(orig_uuid);
}

/// The core streaming loop: keeps the session alive, captures, encodes, and sends.
fn stream_loop(
    client: &mut protocol::EpsonClient,
    grabber: &mut dyn FrameGrabber,
    running: &AtomicBool,
) -> String {
    let mut frame_idx = 0u64;
    let mut last_audio = Instant::now();
    let mut last_auth_heartbeat = Instant::now();
    let frame_budget = Duration::from_micros(1_000_000 / TARGET_FPS);
    let my_ip = client.my_ip;

    while running.load(Ordering::Relaxed) {
        let t0 = Instant::now();

        // 1. Session upkeep first, so it keeps running even while capture fails
        //    (e.g. a Wayland share prompt that is still open).
        if let Err(e) = protocol::drain_auth(&mut client.s_auth, my_ip) {
            return format!("Control channel: {e}");
        }
        if last_auth_heartbeat.elapsed() > Duration::from_secs(30) {
            let _ = client.s_auth.write_all(&protocol::response_0x0108(my_ip));
            last_auth_heartbeat = Instant::now();
        }
        if last_audio.elapsed() > Duration::from_secs(1) {
            last_audio = Instant::now() - AUDIO_SLICE; // too far behind: don't burst
        }
        while last_audio.elapsed() >= AUDIO_SLICE {
            if let Err(e) = protocol::send_keepalive(&mut client.s_aux) {
                return format!("Keepalive: {e}");
            }
            last_audio += AUDIO_SLICE;
        }

        // 2. Capture
        let screen = match grabber.grab() {
            Some(s) => s,
            None => {
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
        };
        let t_capture = t0.elapsed();

        // 3. Encode tiles and build the frame. Windows re-sends the META block
        //    with every keyframe, and so do we (byte-identical to the old template).
        let jpegs: Vec<Vec<u8>> = KEYFRAME_TILES
            .iter()
            .map(|&(x, y, w, h, _, budget)| capture::encode_tile_adaptive(&screen, x, y, w, h, budget))
            .collect();
        let tiles: Vec<VideoTile> = KEYFRAME_TILES
            .iter()
            .zip(&jpegs)
            .map(|(&(x, y, w, h, ts, _), jpeg)| VideoTile { jpeg, x, y, w, h, ts })
            .collect();
        let frame = protocol::build_video_frame(my_ip, &tiles, 4, true);
        let t_encode = t0.elapsed();

        // 4. Send
        if let Err(e) = protocol::send_frame(&mut client.s_video, &frame) {
            return format!("{e}");
        }
        let t_send = t0.elapsed();

        // 5. Frame rate limiter
        let elapsed = t0.elapsed();
        if elapsed < frame_budget {
            std::thread::sleep(frame_budget - elapsed);
        }

        frame_idx += 1;
        if frame_idx <= 5 || frame_idx % 100 == 0 {
            let total_time_ms = t0.elapsed().as_millis().max(1) as f64;
            eprintln!(
                "  Frame {}: cap={}ms enc={}ms send={}ms total={:.0}ms ({:.1}fps, {}KB)",
                frame_idx,
                t_capture.as_millis(),
                (t_encode - t_capture).as_millis(),
                (t_send - t_encode).as_millis(),
                total_time_ms,
                1000.0 / total_time_ms,
                frame.len() / 1024,
            );

            // An all-black frame almost always means the OS blocked the capture.
            if screen.iter().take(10000).all(|&b| b == 0) {
                eprintln!("\n[!] WARNING: the captured frame is entirely black.");
                eprintln!("    -> On Wayland, approve the screen-share prompt when it appears.");
                eprintln!("    -> On macOS, allow Screen Recording in System Settings > Privacy.\n");
            }
        }
    }

    "Stopped".to_string()
}
