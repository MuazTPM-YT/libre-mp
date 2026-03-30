use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Cursor, Read, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::time::Duration;

use crate::hex;

const PROJECTOR_IP: &str = "192.168.88.1";
const PORT_CONTROL: u16 = 3620;
const PORT_VIDEO: u16 = 3621;
fn get_local_ip() -> Ipv4Addr {
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


/// Single recv call — reads whatever is available right now (up to 4096).
/// Matches the Python `socket.recv(1024)` pattern.
fn recv_one(stream: &mut TcpStream, timeout: Duration) -> Vec<u8> {
    stream.set_read_timeout(Some(timeout)).ok();
    let mut buf = vec![0u8; 4096];
    match stream.read(&mut buf) {
        Ok(0) => Vec::new(),
        Ok(n) => buf[..n].to_vec(),
        Err(_) => Vec::new(),
    }
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
pub struct EpsonClient {
    pub my_ip: Ipv4Addr,
    pub proj_ip: Ipv4Addr,
    pub s_auth: TcpStream,
    pub s_video: TcpStream,
    pub s_aux: TcpStream,
}

impl EpsonClient {
    pub fn connect() -> io::Result<Self> {
        let my_ip = get_local_ip();
        let proj_ip: Ipv4Addr = PROJECTOR_IP.parse().unwrap();
        eprintln!("[*] Local IP: {my_ip}, Projector: {proj_ip}");

        // ── 1. Registration ──────────────────────────────────────────────
        eprintln!("[*] 1. Registration on port {PORT_CONTROL}...");
        let mut s_auth = TcpStream::connect((PROJECTOR_IP, PORT_CONTROL))?;
        s_auth.set_nodelay(true)?;
        s_auth.set_read_timeout(Some(Duration::from_secs(5)))?;
        s_auth.write_all(&registration_payload(my_ip))?;

        // Python: recv resp1, then try recv resp2 with 1s timeout
        let resp1 = recv_one(&mut s_auth, Duration::from_secs(5));
        eprintln!("[+]    Registration Resp 1: {} bytes", resp1.len());
        let resp2 = recv_one(&mut s_auth, Duration::from_secs(1));
        if !resp2.is_empty() {
            eprintln!("[+]    Registration Resp 2: {} bytes", resp2.len());
        }

        // Close registration connection and open fresh auth connection
        eprintln!("[*]    Closing Registration channel, opening Auth channel...");
        drop(s_auth);
        std::thread::sleep(Duration::from_millis(100));

        // ── 2. Authentication ────────────────────────────────────────────
        eprintln!("[*]    Authenticating...");
        let mut s_auth = TcpStream::connect((PROJECTOR_IP, PORT_CONTROL))?;
        s_auth.set_nodelay(true)?;
        s_auth.set_read_timeout(Some(Duration::from_secs(5)))?;
        s_auth.write_all(&auth_payload(my_ip, proj_ip))?;

        // Python: single recv(1024) for auth response
        let auth_resp = recv_one(&mut s_auth, Duration::from_secs(5));
        if auth_resp.len() >= 50 {
            eprintln!("[+]    Auth status: 0x{:02x}", auth_resp[50]);
        }
        if auth_resp.len() == 296 {
            eprintln!("[+]    Perfect 296-byte auth response! We are IN.");
        } else {
            eprintln!("[*]    Auth response: {} bytes", auth_resp.len());
        }

        // ── 3. Post-auth handshake ───────────────────────────────────────
        eprintln!("[*]    Post-auth handshake...");
        s_auth.set_read_timeout(Some(Duration::from_secs(3))).ok();
        let mut responded = false;
        let mut ready = false;

        for _ in 0..10 {
            let mut buf = vec![0u8; 4096];
            match s_auth.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = &buf[..n];
                    eprintln!("[*]    Post-auth recv: {} bytes", n);
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

                        eprintln!("[+]    Post-auth cmd=0x{cmd:04x}, {} bytes", msg_len);

                        if cmd == 0x010E && !responded {
                            s_auth.write_all(&response_0x0108(my_ip))?;
                            responded = true;
                            eprintln!("[+]    Sent 0x0108 response");
                        } else if cmd == 0x010E && responded {
                            eprintln!("[*]    Ignoring subsequent 0x010E");
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

        s_auth.set_read_timeout(Some(Duration::from_secs(5))).ok();

        if ready {
            eprintln!("[+]    Post-auth handshake complete. Projector is ready!");
        } else {
            eprintln!("[*]    No explicit ready signal, continuing...");
        }

        // ── 4. Open video channels ───────────────────────────────────────
        eprintln!("[*] 2. Opening video channels on port {PORT_VIDEO}...");
        std::thread::sleep(Duration::from_millis(300));

        let mut s_video = TcpStream::connect((PROJECTOR_IP, PORT_VIDEO))?;
        s_video.set_nodelay(true)?;
        s_video.set_read_timeout(None)?; // blocking for streaming
        s_video.write_all(&video_init_ctrl(my_ip))?;
        eprintln!("[+]    Video channel OPEN (byte28=0x00)");

        let mut s_aux = TcpStream::connect((PROJECTOR_IP, PORT_VIDEO))?;
        s_aux.set_nodelay(true)?;
        s_aux.set_read_timeout(None)?;
        s_aux.write_all(&video_init_data(my_ip))?;
        eprintln!("[+]    Aux channel OPEN (byte28=0x01)");

        // ── 5. Wait for 0x0016 ───────────────────────────────────────────
        eprintln!("[*] 3. Waiting for 0x0016 streaming signal...");
        let data = recv_one(&mut s_auth, Duration::from_secs(10));
        if data.len() >= 20 && &data[..8] == b"EEMP0100" {
            let mut c = Cursor::new(&data[12..16]);
            let cmd = c.read_u32::<LittleEndian>().unwrap();
            eprintln!("[+]    Received cmd=0x{cmd:04x} ({} bytes)", data.len());
        } else {
            eprintln!("[*]    No 0x0016 received, continuing...");
        }

        // ── 6. Warmup buffers ────────────────────────────────────────────
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

/// Send keepalive on aux channel (prevents projector RST timeout).
/// Windows PCAP shows 2646+1764 zero buffers sent periodically.
pub fn send_keepalive(s_aux: &mut TcpStream) -> io::Result<()> {
    s_aux.write_all(&aux_header(2646))?;
    s_aux.write_all(&vec![0u8; 2646])?;
    s_aux.write_all(&aux_header(1764))?;
    s_aux.write_all(&vec![0u8; 1764])?;
    Ok(())
}

/// Drain and respond to projector heartbeat queries on auth channel (port 3620).
/// The projector sends periodic 0x010E queries during streaming.
/// If we don't respond, it RSTs the connection after ~50 seconds.
pub fn drain_auth(s_auth: &mut TcpStream, my_ip: Ipv4Addr) {
    // Non-blocking read
    s_auth.set_read_timeout(Some(Duration::from_millis(1))).ok();
    let mut buf = vec![0u8; 4096];
    match s_auth.read(&mut buf) {
        Ok(0) => {}
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

                if cmd == 0x010E {
                    // Respond with 0x0108
                    let _ = s_auth.write_all(&response_0x0108(my_ip));
                }
                offset += msg_len;
            }
        }
        Err(_) => {} // timeout = no data = fine
    }
}

/// Send video frame data. Uses write_all for maximum throughput.
/// TCP handles segmentation naturally. Frame limiter in main.rs
/// prevents buffer buildup.
pub fn send_frame(stream: &mut TcpStream, data: &[u8]) -> io::Result<()> {
    stream.write_all(data)?;
    Ok(())
}

// ─── Custom EPRD Frame Builder ───────────────────────────────────────────────
// Builds frames from scratch — no template needed, no COM padding, no gray boxes.

/// 46-byte display config (from Windows PCAP). Sent once before first JPEG frame.
#[allow(dead_code)]
const META_DISPLAY_CONFIG: [u8; 46] = [
    0xcc, 0x00, 0x00, 0x00, 0x04, 0x00, 0x03, 0x00,
    0x20, 0x20, 0x00, 0x01, 0xff, 0x00, 0xff, 0x00,
    0xff, 0x00, 0x10, 0x08, 0x00, 0x00, 0x00, 0x00,
    0x06, 0x40, 0x03, 0x84, 0x00, 0x00, 0x00, 0x60,
    0x04, 0x00, 0x02, 0x40, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Build a complete EPRD video frame from raw JPEG tile data.
/// `tiles`: slice of (jpeg_bytes, x, y, w, h) for each tile.
/// `first_frame`: if true, prepends the META display config block.
#[allow(dead_code)]
pub fn build_video_frame(
    my_ip: Ipv4Addr,
    tiles: &[(&[u8], u16, u16, u16, u16)],
    first_frame: bool,
) -> Vec<u8> {
    let ip = my_ip.octets();
    let mut buf = Vec::with_capacity(if first_frame { 16384 } else { 16384 });

    // First frame: prepend META EPRD block
    if first_frame {
        buf.extend_from_slice(b"EPRD0600");
        buf.extend_from_slice(&ip);
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&(META_DISPLAY_CONFIG.len() as u32).to_le_bytes()); // LE for meta
        buf.extend_from_slice(&META_DISPLAY_CONFIG);
    }

    // Build JPEG payload: frame_type(4) + N × (region(16) + jpeg_data)
    let ts = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        & 0xFFFFFFFF) as u32;

    let mut payload = Vec::new();
    payload.extend_from_slice(&4u32.to_be_bytes()); // frame_type = 4 (full frame)

    for &(jpeg, x, y, w, h) in tiles {
        // Region descriptor: x, y, w, h (BE u16), flags (BE u32 = 7), timestamp (BE u32)
        payload.extend_from_slice(&x.to_be_bytes());
        payload.extend_from_slice(&y.to_be_bytes());
        payload.extend_from_slice(&w.to_be_bytes());
        payload.extend_from_slice(&h.to_be_bytes());
        payload.extend_from_slice(&0x00000007u32.to_be_bytes());
        payload.extend_from_slice(&ts.to_be_bytes());
        payload.extend_from_slice(jpeg);
    }

    // JPEG EPRD header (size is BIG-endian, as confirmed in PCAP)
    buf.extend_from_slice(b"EPRD0600");
    buf.extend_from_slice(&ip);
    buf.extend_from_slice(&0u32.to_be_bytes());
    buf.extend_from_slice(&(payload.len() as u32).to_be_bytes()); // BE for jpeg
    buf.extend_from_slice(&payload);

    buf
}


