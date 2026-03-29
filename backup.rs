use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Cursor, Read, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::process::Command;
use std::time::{Duration, Instant};
use turbojpeg::{Compressor, Image, PixelFormat, Subsamp};

// ─── Constants ───────────────────────────────────────────────────────────────

const PROJECTOR_IP: &str = "192.168.88.1";
const PORT_CONTROL: u16 = 3620;
const PORT_VIDEO: u16 = 3621;
const CHUNK_SIZE: usize = 1460;
const CHUNK_DELAY: Duration = Duration::from_millis(2);
const TARGET_SSID: &str = "RESEARCHLAB-fE8DSypQz51AR2Q";
const WIFI_PASSWORD: &str = "A4D73CCDAF45";
const STREAM_W: u32 = 1024;
const STREAM_H: u32 = 768;
const JPEG_QUALITY: i32 = 50;

// ─── Template Slot ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct JpegSlot {
    offset: usize,  // absolute byte offset of JPEG data in template
    size: usize,     // original JPEG byte length
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

// ─── Template Parser ─────────────────────────────────────────────────────────

struct Template {
    buf: Vec<u8>,
    slots: Vec<JpegSlot>,
}

impl Template {
    fn load(path: &str) -> io::Result<Self> {
        let buf = std::fs::read(path)?;
        let mut slots = Vec::new();
        let mut pos = 0;

        while pos + 20 <= buf.len() {
            if &buf[pos..pos + 8] != b"EPRD0600" {
                break;
            }

            let first_payload_byte = buf[pos + 20];
            let payload_size = if first_payload_byte == 0xCC {
                // META block: size is little-endian
                let mut c = Cursor::new(&buf[pos + 16..pos + 20]);
                c.read_u32::<LittleEndian>().unwrap() as usize
            } else {
                // JPEG block: size is big-endian
                let mut c = Cursor::new(&buf[pos + 16..pos + 20]);
                c.read_u32::<BigEndian>().unwrap() as usize
            };

            if first_payload_byte == 0xCC {
                pos += 20 + payload_size;
                continue;
            }

            let payload_start = pos + 20;
            let mut tp: usize = 4; // skip frame_type (4 bytes)

            while tp + 16 <= payload_size {
                let mut c = Cursor::new(&buf[payload_start + tp..payload_start + tp + 8]);
                let x = c.read_u16::<BigEndian>().unwrap();
                let y = c.read_u16::<BigEndian>().unwrap();
                let w = c.read_u16::<BigEndian>().unwrap();
                let h = c.read_u16::<BigEndian>().unwrap();

                if w == 0 || h == 0 || w > 2000 {
                    break;
                }

                tp += 16; // skip region descriptor (16 bytes)

                let abs_jpeg_start = payload_start + tp;
                if abs_jpeg_start + 2 > buf.len()
                    || buf[abs_jpeg_start] != 0xFF
                    || buf[abs_jpeg_start + 1] != 0xD8
                {
                    break;
                }

                // Find FFD9 end marker
                let jpeg_end = find_ffd9(&buf, abs_jpeg_start);
                if jpeg_end == 0 {
                    break;
                }

                let jpeg_size = jpeg_end - abs_jpeg_start;
                slots.push(JpegSlot {
                    offset: abs_jpeg_start,
                    size: jpeg_size,
                    x,
                    y,
                    w,
                    h,
                });

                tp = jpeg_end - payload_start;
            }

            pos += 20 + payload_size;
        }

        eprintln!(
            "[*] Template: {} bytes, {} JPEG slots",
            buf.len(),
            slots.len()
        );

        Ok(Template { buf, slots })
    }

    fn swap(&mut self, idx: usize, padded_jpeg: &[u8]) {
        let slot = &self.slots[idx];
        debug_assert_eq!(padded_jpeg.len(), slot.size);
        self.buf[slot.offset..slot.offset + slot.size].copy_from_slice(padded_jpeg);
    }
}

fn find_ffd9(buf: &[u8], start: usize) -> usize {
    let mut i = start;
    while i + 1 < buf.len() {
        if buf[i] == 0xFF && buf[i + 1] == 0xD9 {
            return i + 2;
        }
        i += 1;
    }
    0
}

// ─── JPEG Padding ────────────────────────────────────────────────────────────

fn pad_jpeg(jpeg: &[u8], target_size: usize) -> Vec<u8> {
    if jpeg.len() >= target_size {
        // Truncate: replace last 2 bytes with FFD9
        let mut out = jpeg[..target_size].to_vec();
        let end = out.len();
        out[end - 2] = 0xFF;
        out[end - 1] = 0xD9;
        return out;
    }

    // Find last FFD9
    let ffd9_pos = jpeg
        .windows(2)
        .rposition(|w| w[0] == 0xFF && w[1] == 0xD9)
        .unwrap_or(jpeg.len());

    let padding_needed = target_size - jpeg.len();
    let mut out = Vec::with_capacity(target_size);
    out.extend_from_slice(&jpeg[..ffd9_pos]);

    // Insert COM markers (FF FE + u16 length + null padding)
    let mut remaining = padding_needed;
    while remaining > 0 {
        if remaining >= 4 {
            let chunk = remaining.min(65533 + 4) - 4;
            out.push(0xFF);
            out.push(0xFE);
            let length = (chunk + 2) as u16;
            out.push((length >> 8) as u8);
            out.push((length & 0xFF) as u8);
            out.extend(std::iter::repeat(0u8).take(chunk));
            remaining -= 4 + chunk;
        } else {
            out.extend(std::iter::repeat(0u8).take(remaining));
            remaining = 0;
        }
    }

    out.extend_from_slice(&jpeg[ffd9_pos..]);
    debug_assert_eq!(out.len(), target_size);
    out
}

// ─── Wi-Fi ───────────────────────────────────────────────────────────────────

fn wifi_connect() -> Option<String> {
    // Get current connection UUID for later restoration
    let current = Command::new("nmcli")
        .args(["-t", "-f", "UUID", "connection", "show", "--active"])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8(o.stdout)
                .ok()
                .map(|s| s.lines().next().unwrap_or("").to_string())
        })
        .filter(|s| !s.is_empty());

    eprintln!("[*] Auto-connecting to {TARGET_SSID}...");

    // Delete any previous connection profile for this SSID
    let _ = Command::new("nmcli")
        .args(["connection", "delete", TARGET_SSID])
        .output();

    // Scan first
    let _ = Command::new("nmcli")
        .args(["device", "wifi", "rescan"])
        .output();
    std::thread::sleep(Duration::from_secs(2));

    // Connect with the MAC address as password
    let status = Command::new("nmcli")
        .args([
            "device",
            "wifi",
            "connect",
            TARGET_SSID,
            "password",
            WIFI_PASSWORD,
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            eprintln!("[+] Connected to {TARGET_SSID}!");
            current
        }
        _ => {
            eprintln!("[-] Failed to connect to {TARGET_SSID}");
            std::process::exit(1);
        }
    }
}

fn wifi_restore(orig_uuid: Option<String>) {
    eprintln!("\n[*] Restoring original Wi-Fi...");
    // Delete the projector connection profile
    let _ = Command::new("nmcli")
        .args(["connection", "delete", TARGET_SSID])
        .output();

    if let Some(uuid) = orig_uuid {
        let _ = Command::new("nmcli")
            .args(["connection", "up", &uuid])
            .output();
        eprintln!("[+] Network restored.");
    }
}

// ─── Networking Helpers ──────────────────────────────────────────────────────

fn get_local_ip() -> Ipv4Addr {
    // Use a UDP socket to determine the routable local IP
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").expect("bind");
    sock.connect((PROJECTOR_IP, 80)).expect("connect");
    match sock.local_addr().expect("local_addr").ip() {
        std::net::IpAddr::V4(ip) => ip,
        _ => Ipv4Addr::new(192, 168, 88, 2),
    }
}

fn ip_bytes(ip: Ipv4Addr) -> [u8; 4] {
    ip.octets()
}

fn ip_bytes_rev(ip: Ipv4Addr) -> [u8; 4] {
    let o = ip.octets();
    [o[3], o[2], o[1], o[0]]
}

fn send_chunked(stream: &mut TcpStream, data: &[u8]) -> io::Result<()> {
    for chunk in data.chunks(CHUNK_SIZE) {
        stream.write_all(chunk)?;
        std::thread::sleep(CHUNK_DELAY);
    }
    Ok(())
}

fn recv_all(stream: &mut TcpStream, timeout: Duration) -> Vec<u8> {
    stream
        .set_read_timeout(Some(timeout))
        .expect("set_read_timeout");
    let mut buf = vec![0u8; 4096];
    let mut result = Vec::new();
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                result.extend_from_slice(&buf[..n]);
                // Try to read more with short timeout
                stream
                    .set_read_timeout(Some(Duration::from_millis(200)))
                    .ok();
            }
            Err(_) => break,
        }
    }
    result
}

// ─── Protocol Payloads ───────────────────────────────────────────────────────

fn registration_payload(my_ip: Ipv4Addr) -> Vec<u8> {
    let mut p = Vec::with_capacity(68);
    p.extend_from_slice(b"EEMP0100");
    p.extend_from_slice(&ip_bytes(my_ip));
    p.extend_from_slice(&hex::decode("0200000030000000007f0000b0f8ef5314000000").unwrap());
    p.extend_from_slice(&[0u8; 32]);
    p
}

fn auth_payload(my_ip: Ipv4Addr, proj_ip: Ipv4Addr) -> Vec<u8> {
    let my = ip_bytes(my_ip);
    let proj = ip_bytes(proj_ip);
    let mac = hex::decode("a4d73ccdaf45").unwrap();

    let mut p = Vec::with_capacity(264);
    p.extend_from_slice(b"EEMP0100");
    p.extend_from_slice(&my);
    p.extend_from_slice(
        &hex::decode("01010000f40000000101000000380f000000000000ffffff0000000000020f0b0004000320200001ff00ff00ff00000810000000010e0000").unwrap(),
    );
    p.extend_from_slice(&mac);
    p.extend_from_slice(&[0u8; 16]);
    p.extend_from_slice(&proj);
    p.extend_from_slice(
        &hex::decode("a600000005000000380000000200000004000000").unwrap(),
    );
    p.extend_from_slice(&my);
    p.extend_from_slice(
        &hex::decode("0c00000004000000000000000100000004000000500043000b00000004000000000000001c00000000000000040000003600000001000000030000002a000000").unwrap(),
    );
    p.extend_from_slice(&mac);
    p.extend_from_slice(&proj);
    // "RESEARCHLAB" + null padding to 32 bytes
    let mut ssid_buf = [0u8; 32];
    let ssid = b"RESEARCHLAB";
    ssid_buf[..ssid.len()].copy_from_slice(ssid);
    p.extend_from_slice(&ssid_buf);
    p.extend_from_slice(
        &hex::decode("0f00000004000000320000000d000000040000000200000026000000080000000010000000100000").unwrap(),
    );
    p
}

fn response_0x0108(my_ip: Ipv4Addr) -> Vec<u8> {
    let pcap_hex = concat!(
        "45454d5030313030c0a858020801000048010000",
        "0001000000000000000000000000000000000000",
        "00000000000000000000000000000000000000000000000000000000",
        "1401000005000000380000000200000004000000",
        "c0a858020c00000004000000010000000100000004000000",
        "500043000b00000004000000000000001c00000000000000",
        "07000000440000000100000005000000380000000200000004000000",
        "c0a858020c00000004000000010000000100000004000000",
        "500043000b00000004000000000000001c00000000000000",
        "08000000800000000400000005000000380000000200000004000000",
        "c0a858020c00000004000000010000000100000004000000",
        "500043000b00000004000000010100001c00000000000000",
        "000000000c000000020000000400000002000000",
        "000000000c000000020000000400000003000000",
        "000000000c000000020000000400000004000000",
    );
    let mut raw = hex::decode(pcap_hex).unwrap();
    let old_ip: [u8; 4] = [192, 168, 88, 2];
    let new_ip = ip_bytes(my_ip);
    // Replace all occurrences of old IP
    let mut i = 0;
    while i + 3 < raw.len() {
        if raw[i..i + 4] == old_ip {
            raw[i..i + 4].copy_from_slice(&new_ip);
            i += 4;
        } else {
            i += 1;
        }
    }
    raw
}

fn video_init_ctrl(my_ip: Ipv4Addr) -> Vec<u8> {
    let mut p = Vec::with_capacity(36);
    p.extend_from_slice(b"EPRD0600");
    p.extend_from_slice(&ip_bytes(my_ip));
    p.extend_from_slice(&hex::decode("0000000010000000d0000000").unwrap());
    p.extend_from_slice(&ip_bytes_rev(my_ip));
    p.extend_from_slice(&[0u8; 8]);
    p
}

fn video_init_data(my_ip: Ipv4Addr) -> Vec<u8> {
    let mut p = Vec::with_capacity(36);
    p.extend_from_slice(b"EPRD0600");
    p.extend_from_slice(&ip_bytes(my_ip));
    p.extend_from_slice(&hex::decode("0000000010000000d0000000").unwrap());
    p.extend_from_slice(&ip_bytes_rev(my_ip));
    // byte 28 = 0x01 for data channel
    p.push(0x01);
    p.extend_from_slice(&[0u8; 7]);
    p
}

fn aux_header(size: u32) -> Vec<u8> {
    let mut h = Vec::with_capacity(5);
    h.push(0xC9);
    h.write_u32::<LittleEndian>(size).unwrap();
    h
}

// ─── Protocol Client ─────────────────────────────────────────────────────────

#[allow(dead_code)]
struct EpsonClient {
    my_ip: Ipv4Addr,
    proj_ip: Ipv4Addr,
    s_auth: TcpStream,
    s_video: TcpStream,
    s_aux: TcpStream,
}

impl EpsonClient {
    fn connect() -> io::Result<Self> {
        let my_ip = get_local_ip();
        let proj_ip: Ipv4Addr = PROJECTOR_IP.parse().unwrap();
        eprintln!("[*] Local IP: {my_ip}, Projector: {proj_ip}");

        // 1. Registration
        eprintln!("[*] 1. Registration on port {PORT_CONTROL}...");
        let mut s_auth = TcpStream::connect((PROJECTOR_IP, PORT_CONTROL))?;
        s_auth.set_nodelay(true)?;
        s_auth.write_all(&registration_payload(my_ip))?;
        let _resp = recv_all(&mut s_auth, Duration::from_secs(3));
        eprintln!("[+]    Registration OK");
        drop(s_auth);

        std::thread::sleep(Duration::from_millis(100));

        // 2. Authentication
        eprintln!("[*]    Authenticating...");
        let mut s_auth = TcpStream::connect((PROJECTOR_IP, PORT_CONTROL))?;
        s_auth.set_nodelay(true)?;
        s_auth.write_all(&auth_payload(my_ip, proj_ip))?;

        let resp = recv_all(&mut s_auth, Duration::from_secs(5));
        if resp.len() >= 50 {
            eprintln!("[+]    Auth status: 0x{:02x}", resp[50]);
        }
        if resp.len() == 296 {
            eprintln!("[+]    Perfect 296-byte auth response! We are IN.");
        } else {
            eprintln!("[*]    Auth response: {} bytes", resp.len());
        }

        // 3. Post-auth handshake
        eprintln!("[*]    Post-auth handshake...");
        s_auth
            .set_read_timeout(Some(Duration::from_secs(3)))
            .ok();
        let mut responded = false;
        let mut ready = false;

        for _ in 0..10 {
            let mut buf = vec![0u8; 4096];
            match s_auth.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = &buf[..n];
                    let mut offset = 0;
                    while offset + 20 <= data.len() {
                        if &data[offset..offset + 8] != b"EEMP0100" {
                            break;
                        }
                        let mut c = Cursor::new(&data[offset + 12..offset + 16]);
                        let cmd = c.read_u32::<LittleEndian>().unwrap();
                        let mut c = Cursor::new(&data[offset + 16..offset + 20]);
                        let payload_len = c.read_u32::<LittleEndian>().unwrap() as usize;
                        let msg_len = 20 + payload_len;

                        if cmd == 0x010E && !responded {
                            s_auth.write_all(&response_0x0108(my_ip))?;
                            responded = true;
                            eprintln!("[+]    Sent 0x0108 response");
                        } else if cmd == 0x0110 {
                            eprintln!("[+]    Received 0x0110 'Ready to Stream'!");
                            ready = true;
                            break;
                        }
                        offset += msg_len;
                    }
                    if ready {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        if !ready {
            eprintln!("[*]    No explicit ready signal, continuing...");
        }

        // 4. Open video channels
        eprintln!("[*] 2. Opening video channels on port {PORT_VIDEO}...");
        std::thread::sleep(Duration::from_millis(300));

        let mut s_video = TcpStream::connect((PROJECTOR_IP, PORT_VIDEO))?;
        s_video.set_nodelay(true)?;
        s_video.write_all(&video_init_ctrl(my_ip))?;
        eprintln!("[+]    Video channel OPEN (byte28=0x00)");

        let mut s_aux = TcpStream::connect((PROJECTOR_IP, PORT_VIDEO))?;
        s_aux.set_nodelay(true)?;
        s_aux.write_all(&video_init_data(my_ip))?;
        eprintln!("[+]    Aux channel OPEN (byte28=0x01)");

        // 5. Wait for 0x0016
        eprintln!("[*] 3. Waiting for 0x0016 streaming signal...");
        s_auth
            .set_read_timeout(Some(Duration::from_secs(10)))
            .ok();
        let mut buf = vec![0u8; 4096];
        match s_auth.read(&mut buf) {
            Ok(n) if n >= 20 && &buf[..8] == b"EEMP0100" => {
                let mut c = Cursor::new(&buf[12..16]);
                let cmd = c.read_u32::<LittleEndian>().unwrap();
                eprintln!("[+]    Received cmd=0x{cmd:04x} ({n} bytes)");
            }
            _ => eprintln!("[*]    No 0x0016 received, continuing..."),
        }

        // 6. Warmup buffers
        eprintln!("[*] 4. Sending warmup buffers...");
        for size in [7276u32, 2646, 1764] {
            s_aux.write_all(&aux_header(size))?;
            s_aux.write_all(&vec![0u8; size as usize])?;
            eprintln!("[+]    Warmup: {size} zeros");
            std::thread::sleep(Duration::from_millis(50));
        }
        std::thread::sleep(Duration::from_millis(500));

        eprintln!("\n[+] BINGO! Ready for video stream!");

        Ok(EpsonClient {
            my_ip,
            proj_ip,
            s_auth,
            s_video,
            s_aux,
        })
    }
}

// ─── Screen Capture ──────────────────────────────────────────────────────────

fn capture_screen() -> Option<image::RgbaImage> {
    let output = Command::new("grim").args(["-t", "png", "-"]).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let img = image::load_from_memory_with_format(&output.stdout, image::ImageFormat::Png).ok()?;
    let resized = img.resize_exact(STREAM_W, STREAM_H, image::imageops::FilterType::Triangle);
    Some(resized.to_rgba8())
}

// ─── JPEG Encoding ──────────────────────────────────────────────────────────

fn encode_tile(
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

// ─── Main ────────────────────────────────────────────────────────────────────

fn main() {
    eprintln!("=== Epson EasyMP Rust Streamer ===\n");

    // 1. Auto-connect Wi-Fi
    let orig_uuid = wifi_connect();

    // Setup Ctrl+C handler for clean Wi-Fi restoration
    let orig_uuid_clone = orig_uuid.clone();
    ctrlc_handler(orig_uuid_clone);

    // 2. Connect to projector
    let mut client = match EpsonClient::connect() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[-] Connection failed: {e}");
            wifi_restore(orig_uuid);
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
        wifi_restore(orig_uuid);
        std::process::exit(1);
    };

    let mut tpl = match Template::load(template_path.to_str().unwrap()) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[-] Template load failed: {e}");
            wifi_restore(orig_uuid);
            std::process::exit(1);
        }
    };

    // 4. Initialize turbojpeg compressor
    let mut compressor = Compressor::new().expect("turbojpeg init");
    let _ = compressor.set_quality(JPEG_QUALITY);
    let _ = compressor.set_subsamp(Subsamp::Sub2x2); // 4:2:0

    eprintln!(
        "\n[*] Starting live stream — {} slots, {}×{} viewport",
        tpl.slots.len(),
        STREAM_W,
        STREAM_H
    );

    // 5. Streaming loop
    let mut frame_idx = 0u64;
    let mut jpeg_cache: std::collections::HashMap<(u16, u16, u16, u16), Vec<u8>> =
        std::collections::HashMap::new();

    loop {
        let t0 = Instant::now();

        // Capture screen
        let screen = match capture_screen() {
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
                .or_insert_with(|| encode_tile(&mut compressor, &screen, slot.x, slot.y, slot.w, slot.h))
                .clone();

            let padded = pad_jpeg(&jpeg_raw, slot.size);
            tpl.swap(idx, &padded);
        }
        let t_encode = t0.elapsed();

        // Send template through TCP chunker
        match send_chunked(&mut client.s_video, &tpl.buf) {
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
    wifi_restore(orig_uuid);
}

fn ctrlc_handler(orig_uuid: Option<String>) {
    // Use a simple approach: register a handler that restores Wi-Fi on ctrl+c
    let _ = std::thread::spawn(move || {
        // Wait for ctrl+c signal
        let mut sigs =
            signal_hook::iterator::Signals::new(&[signal_hook::consts::SIGINT]).unwrap();
        for _ in sigs.forever() {
            wifi_restore(orig_uuid);
            std::process::exit(0);
        }
    });
}

mod hex {
    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        if s.len() % 2 != 0 {
            return Err("Odd length hex string".into());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| format!("Hex decode: {e}"))
            })
            .collect()
    }
}
