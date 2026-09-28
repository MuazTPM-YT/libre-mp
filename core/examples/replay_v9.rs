//! Replay captured windows video blocks to a projector, to split "our frames bad" from "our session bad".
//! usage: cargo run -p libremp-core --example replay_v9 -- <projector-ip> <video.bin>
//! video.bin = raw client->projector bytes of the windows 3621 stream (EPRD blocks back to back).

use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

use libremp_core::protocol::{self, EpsonClient};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ip: Ipv4Addr = args.get(1).and_then(|a| a.parse().ok()).expect("usage: replay_v9 <projector-ip> <video.bin>");
    let data = std::fs::read(args.get(2).expect("usage: replay_v9 <projector-ip> <video.bin>")).expect("read video.bin");

    // split into EPRD blocks; META size is le, jpeg size is be
    let mut blocks = Vec::new();
    let mut i = 0;
    while i + 20 <= data.len() && &data[i..i + 8] == b"EPRD0600" {
        let le = u32::from_le_bytes(data[i + 16..i + 20].try_into().unwrap()) as usize;
        let be = u32::from_be_bytes(data[i + 16..i + 20].try_into().unwrap()) as usize;
        let size = if le < 1 << 20 { le } else { be };
        if i + 20 + size > data.len() {
            break;
        }
        blocks.push(data[i..i + 20 + size].to_vec());
        i += 20 + size;
    }
    eprintln!("[*] {} complete blocks in capture", blocks.len());

    let mut client = EpsonClient::connect("", "", Some(ip), None).expect("connect");
    let my_ip = client.my_ip;
    let use_v9 = protocol::is_v9(client.version);
    // sender ip in each header -> ours
    for b in &mut blocks {
        b[8..12].copy_from_slice(&my_ip.octets());
    }

    let start = Instant::now();
    let mut last_heartbeat = Instant::now();
    let mut next = 0;
    while start.elapsed() < Duration::from_secs(30) {
        if let Err(e) = protocol::drain_auth(&mut client.s_auth, my_ip) {
            eprintln!("[-] control: {e}");
            break;
        }
        if last_heartbeat.elapsed() > Duration::from_secs(5) {
            let _ = std::io::Write::write_all(&mut client.s_auth, &protocol::control_heartbeat(my_ip, use_v9));
            last_heartbeat = Instant::now();
        }
        if next < blocks.len() {
            match protocol::send_frame(&mut client.s_video, &blocks[next]) {
                Ok(()) => eprintln!("  t={:.2}s block {next} ({} bytes)", start.elapsed().as_secs_f32(), blocks[next].len()),
                Err(e) => {
                    eprintln!("[-] video send failed at block {next}: {e}");
                    break;
                }
            }
            next += 1;
        }
        std::thread::sleep(Duration::from_millis(80));
    }
    eprintln!("[*] done after {:.1}s; last picture should still be on screen", start.elapsed().as_secs_f32());
}
