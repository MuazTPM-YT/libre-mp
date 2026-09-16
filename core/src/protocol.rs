use byteorder::{LittleEndian, WriteBytesExt};
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;
#[cfg(target_os = "macos")]
use std::os::unix::io::AsRawFd;
#[cfg(target_os = "windows")]
use std::os::windows::io::AsRawSocket;

#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::process::Command;

use crate::hex;

/// Epson's Quick Connect / Simple AP address, tried last when nothing else answers.
const DEFAULT_PROJECTOR_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 88, 1);
const PORT_CONTROL: u16 = 3620;
const PORT_VIDEO: u16 = 3621;
/// Bounded connects: a wrong address must fail in seconds, not the OS's ~2 minutes.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

/// EEMP commands seen in the Windows iProjection capture.
const CMD_REGISTER: u32 = 0x0002;
const CMD_REGISTER_INFO: u32 = 0x0003;
const CMD_REGISTER_MAC: u32 = 0x0015;
const CMD_AUTH_OK: u32 = 0x0102;
const CMD_DISCONNECT: u32 = 0x0104;
const CMD_STATUS_QUERY: u32 = 0x010E;
const CMD_READY: u32 = 0x0110;

/// All IPv4 default gateways, in routing-table order.
///
/// In Epson Simple AP / Quick Connect mode the projector is the DHCP server and
/// hands itself out as the router (confirmed in the captures), so its address is
/// a default gateway. We return *all* of them because a laptop that is also on
/// Ethernet or a VPN has several, and the first one is often not the projector.
fn default_gateways() -> Vec<Ipv4Addr> {
    let mut out = Vec::new();
    #[cfg(target_os = "linux")]
    {
        // /proc/net/route: destination 0.0.0.0 with RTF_GATEWAY (0x2); hex, little-endian.
        if let Ok(table) = std::fs::read_to_string("/proc/net/route") {
            for line in table.lines().skip(1) {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if cols.len() < 4 || cols[1] != "00000000" {
                    continue;
                }
                let flags = u16::from_str_radix(cols[3], 16).unwrap_or(0);
                if let Ok(gw) = u32::from_str_radix(cols[2], 16) {
                    if gw != 0 && flags & 0x2 != 0 {
                        out.push(Ipv4Addr::from(gw.to_le_bytes()));
                    }
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        // `netstat -rn -f inet` lists every default route: "default  192.168.88.1  UGScg  en0".
        if let Ok(o) = Command::new("netstat").args(["-rn", "-f", "inet"]).output() {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                let mut cols = line.split_whitespace();
                if cols.next() == Some("default") {
                    if let Some(ip) = cols.next().and_then(|s| s.parse().ok()) {
                        out.push(ip);
                    }
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        // `route print -4` rows are numeric and locale-independent (unlike ipconfig's
        // translated "Default Gateway" label): "0.0.0.0  0.0.0.0  192.168.88.1  192.168.88.2  35".
        if let Ok(o) = Command::new("route").args(["print", "-4"]).output() {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if cols.len() >= 3 && cols[0] == "0.0.0.0" && cols[1] == "0.0.0.0" {
                    if let Ok(ip) = cols[2].parse() {
                        out.push(ip);
                    }
                }
            }
        }
    }
    out
}

/// Where to look for the projector: explicit address (e.g. from the QR code)
/// first, then every default gateway, then the Epson Simple AP default.
fn projector_candidates(override_ip: Option<Ipv4Addr>) -> Vec<Ipv4Addr> {
    let mut all: Vec<Ipv4Addr> = override_ip.into_iter().chain(default_gateways()).collect();
    all.push(DEFAULT_PROJECTOR_IP);
    let mut seen = std::collections::HashSet::new();
    all.retain(|ip| seen.insert(*ip));
    all
}

/// Opens a tuned TCP connection (no Nagle, keepalive) with a bounded connect.
fn open(ip: Ipv4Addr, port: u16) -> io::Result<TcpStream> {
    let s = TcpStream::connect_timeout(&SocketAddr::from((ip, port)), CONNECT_TIMEOUT)?;
    s.set_nodelay(true)?;
    enable_tcp_keepalive(&s);
    Ok(s)
}

/// Single recv call — reads whatever is available right now (up to 4096).
fn recv_one(stream: &mut TcpStream, timeout: Duration) -> Vec<u8> {
    stream.set_read_timeout(Some(timeout)).ok();
    let mut buf = vec![0u8; 4096];
    match stream.read(&mut buf) {
        Ok(n) => buf[..n].to_vec(),
        Err(_) => Vec::new(),
    }
}

/// Splits a buffer into `(cmd, payload)` EEMP messages. Stops at the first
/// non-EEMP byte; a truncated last payload is returned as far as it goes.
fn eemp_messages(data: &[u8]) -> Vec<(u32, &[u8])> {
    let mut out = Vec::new();
    let mut off = 0;
    while off + 20 <= data.len() && &data[off..off + 8] == b"EEMP0100" {
        let cmd = u32::from_le_bytes(data[off + 12..off + 16].try_into().unwrap());
        let len = u32::from_le_bytes(data[off + 16..off + 20].try_into().unwrap()) as usize;
        let end = (off + 20).saturating_add(len);
        out.push((cmd, &data[off + 20..end.min(data.len())]));
        off = end;
    }
    out
}

/// 20-byte EEMP header: magic, sender IP, command, payload length (LE).
fn eemp_header(my_ip: Ipv4Addr, cmd: u32, payload_len: u32) -> Vec<u8> {
    let mut h = Vec::with_capacity(20);
    h.extend_from_slice(b"EEMP0100");
    h.extend_from_slice(&my_ip.octets());
    h.extend_from_slice(&cmd.to_le_bytes());
    h.extend_from_slice(&payload_len.to_le_bytes());
    h
}

/// Parses a MAC written as 12 hex digits, with or without `:`/`-` separators.
fn mac_from_hex(s: &str) -> Option<[u8; 6]> {
    let hex: String = s.chars().filter(|c| *c != ':' && *c != '-').collect();
    if hex.len() != 12 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut mac = [0u8; 6];
    for (i, b) in mac.iter_mut().enumerate() {
        *b = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(mac)
}

// ─── Protocol Payloads ───────────────────────────────────────────────────────

/// Registration request (cmd 0x0002). Byte-identical to Windows iProjection.
pub fn registration_payload(my_ip: Ipv4Addr) -> Vec<u8> {
    let mut p = eemp_header(my_ip, CMD_REGISTER, 48);
    p.extend_from_slice(&hex::decode("007f0000b0f8ef5314000000").unwrap());
    p.extend_from_slice(&[0u8; 36]);
    p
}

/// What the projector says about itself when we register.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ProjectorIdentity {
    /// Projector name (cmd 0x0003), e.g. `RESEARCHLAB`.
    pub name: Option<Vec<u8>>,
    /// Projector MAC (cmd 0x0015, else cmd 0x0003) — the EasyMP auth key.
    pub mac: Option<[u8; 6]>,
}

/// Reads the projector's name and MAC from its registration replies. This is
/// where the Windows client gets them, so no password or SSID guessing is needed.
pub fn parse_registration_response(data: &[u8]) -> ProjectorIdentity {
    let nonzero = |b: &[u8]| -> Option<[u8; 6]> {
        let m: [u8; 6] = b.try_into().ok()?;
        (m != [0; 6]).then_some(m)
    };
    let mut id = ProjectorIdentity::default();
    let mut info_mac = None;
    for (cmd, p) in eemp_messages(data) {
        match cmd {
            CMD_REGISTER_INFO if p.len() >= 36 => {
                let name: Vec<u8> = p[4..36].iter().copied().take_while(|&b| b != 0).collect();
                if !name.is_empty() {
                    id.name = Some(name);
                }
                if p.len() >= 54 {
                    info_mac = nonzero(&p[48..54]);
                }
            }
            CMD_REGISTER_MAC if p.len() >= 6 => id.mac = nonzero(&p[..6]).or(id.mac),
            _ => {}
        }
    }
    id.mac = id.mac.or(info_mac);
    id
}

/// Authentication request (cmd 0x0101). Byte-identical to Windows iProjection
/// when `keyword` is `None`.
///
/// It is a TLV structure. `mac`, `proj_ip` and `name` identify the projector we
/// are connecting to (the values it reported during registration).
///
/// `keyword` is the projector's 4-digit **Projector Keyword**, when that setting
/// is on. It goes into the 16-byte field right after the MAC, which is all
/// zeroes in our capture (that projector had the setting off).
///
/// ponytail: the keyword offset is inferred, not captured — the only session we
/// have was keyword-free. If a keyword projector still refuses the connection,
/// capture Epson iProjection connecting to it and compare the 0x0101 packet
/// against `core/tests/handshake_snapshot.rs`. Everything else here is verified.
pub fn auth_payload(
    my_ip: Ipv4Addr,
    proj_ip: Ipv4Addr,
    mac: &[u8; 6],
    name: &[u8],
    keyword: Option<&str>,
) -> Vec<u8> {
    let my = my_ip.octets();
    let proj = proj_ip.octets();

    let mut p = eemp_header(my_ip, 0x0101, 244);
    p.extend_from_slice(
        &hex::decode("0101000000380f000000000000ffffff0000000000020f0b0004000320200001ff00ff00ff00000810000000010e0000").unwrap(),
    );
    p.extend_from_slice(mac);
    let mut keyword_field = [0u8; 16];
    if let Some(k) = keyword {
        let b = k.as_bytes();
        let n = b.len().min(16);
        keyword_field[..n].copy_from_slice(&b[..n]);
    }
    p.extend_from_slice(&keyword_field);
    p.extend_from_slice(&proj);
    p.extend_from_slice(&hex::decode("a600000005000000380000000200000004000000").unwrap());
    p.extend_from_slice(&my);
    // TLVs: 0x0C=0, 0x01="PC", 0x0B=0, 0x1C=(empty). 0x0B is a fixed tag, not a length.
    p.extend_from_slice(&hex::decode("0c0000000400000000000000010000000400000050004300").unwrap());
    p.extend_from_slice(&hex::decode("0b0000000400000000000000").unwrap());
    p.extend_from_slice(
        &hex::decode("1c00000000000000040000003600000001000000030000002a000000").unwrap(),
    );
    p.extend_from_slice(mac);
    p.extend_from_slice(&proj);
    let mut name_buf = [0u8; 32];
    let n = name.len().min(32);
    name_buf[..n].copy_from_slice(&name[..n]);
    p.extend_from_slice(&name_buf);
    p.extend_from_slice(
        &hex::decode("0f00000004000000320000000d000000040000000200000026000000080000000010000000100000").unwrap(),
    );
    p
}

/// Constructs a response to the projector's 0x010E heartbeat queries.
pub fn response_0x0108(my_ip: Ipv4Addr) -> Vec<u8> {
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
    let new_ip = my_ip.octets();
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

/// Generates the initialization payload for a video-port channel
/// (`channel` 0 = video, 1 = aux/audio).
fn video_init(my_ip: Ipv4Addr, channel: u8) -> Vec<u8> {
    let o = my_ip.octets();
    let mut p = Vec::with_capacity(36);
    p.extend_from_slice(b"EPRD0600");
    p.extend_from_slice(&o);
    p.extend_from_slice(&hex::decode("0000000010000000d0000000").unwrap());
    p.extend_from_slice(&[o[3], o[2], o[1], o[0]]);
    p.push(channel);
    p.extend_from_slice(&[0u8; 7]);
    p
}

/// Generates a header for the auxiliary stream indicating the size of the following buffer.
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
    /// Runs the full EasyMP handshake.
    ///
    /// * `password` — only a fallback auth key (projector MAC as 12 hex digits) for
    ///   projectors that do not report their MAC during registration.
    /// * `ssid` — only a fallback name source (`<name>-<suffix>`).
    /// * `proj_ip_override` — tried first (e.g. the IP from the QR code).
    /// * `keyword` — the projector's 4-digit Projector Keyword, if it shows one.
    pub fn connect(
        password: &str,
        ssid: &str,
        proj_ip_override: Option<Ipv4Addr>,
        keyword: Option<&str>,
    ) -> io::Result<Self> {
        // ── 1. Registration: find the projector on the first address that answers
        let candidates = projector_candidates(proj_ip_override);
        eprintln!("[*] 1. Looking for the projector on port {PORT_CONTROL}: {candidates:?}");
        let mut found = None;
        for ip in &candidates {
            match open(*ip, PORT_CONTROL) {
                Ok(s) => {
                    found = Some((*ip, s));
                    break;
                }
                Err(e) => eprintln!("[*]    {ip}: {e}"),
            }
        }
        let (proj_ip, mut s_reg) = found.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("no Epson projector answered on port {PORT_CONTROL} (tried {candidates:?})"),
            )
        })?;
        // Our address on the interface that actually reaches the projector.
        let my_ip = match s_reg.local_addr()?.ip() {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(_) => return Err(io::Error::new(io::ErrorKind::Unsupported, "IPv6 is not supported")),
        };
        eprintln!("[*] Local IP: {my_ip}, Projector: {proj_ip}");

        s_reg.write_all(&registration_payload(my_ip))?;
        let mut reg = recv_one(&mut s_reg, Duration::from_secs(5));
        reg.extend(recv_one(&mut s_reg, Duration::from_secs(1)));
        eprintln!("[+]    Registration reply: {} bytes", reg.len());
        let id = parse_registration_response(&reg);
        drop(s_reg);

        let given_mac = mac_from_hex(password);
        let mac = match (id.mac, given_mac) {
            (Some(m), g) => {
                if g.is_some_and(|g| g != m) {
                    eprintln!("[*]    Note: password is not the projector MAC; using the MAC the projector reported.");
                }
                m
            }
            (None, Some(g)) => g,
            (None, None) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "projector did not report its MAC, and --password is not a 12-hex-digit MAC",
                ))
            }
        };
        let name = id.name.clone().unwrap_or_else(|| {
            ssid.rsplit_once('-').map_or(ssid, |(n, _)| n).as_bytes().to_vec()
        });
        eprintln!(
            "[+]    Projector: '{}' MAC {}",
            String::from_utf8_lossy(&name),
            mac.iter().map(|b| format!("{b:02x}")).collect::<String>()
        );
        std::thread::sleep(Duration::from_millis(100));

        // ── 2. Authentication ────────────────────────────────────────────
        eprintln!("[*]    Authenticating...");
        let mut s_auth = open(proj_ip, PORT_CONTROL)?;
        s_auth.write_all(&auth_payload(my_ip, proj_ip, &mac, &name, keyword))?;
        let auth_resp = recv_one(&mut s_auth, Duration::from_secs(5));
        // Status byte 51 (payload offset 30) of the 0x0102 reply is 0 on success
        // (confirmed in the Windows capture; polarity per Rhino Security Labs).
        for (cmd, p) in eemp_messages(&auth_resp) {
            eprintln!("[+]    Auth reply cmd=0x{cmd:04x}, {} bytes", p.len() + 20);
            if cmd == CMD_AUTH_OK && p.len() > 30 && p[30] != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    if keyword.is_some() {
                        "projector rejected the connection (wrong keyword, or another device is connected)"
                    } else {
                        "projector rejected the connection — if it shows a 4-digit keyword on screen, pass it with --keyword"
                    },
                ));
            }
        }
        if auth_resp.is_empty() {
            eprintln!("[*]    No auth reply (projector may be busy)");
        }

        // ── 3. Post-auth handshake ───────────────────────────────────────
        eprintln!("[*]    Post-auth handshake...");
        s_auth.set_read_timeout(Some(Duration::from_secs(3))).ok();
        let mut responded = false;
        let mut ready = false;
        for _ in 0..10 {
            let mut buf = vec![0u8; 4096];
            let n = match s_auth.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            for (cmd, p) in eemp_messages(&buf[..n]) {
                eprintln!("[+]    Post-auth cmd=0x{cmd:04x}, {} bytes", p.len() + 20);
                if cmd == CMD_STATUS_QUERY && !responded {
                    s_auth.write_all(&response_0x0108(my_ip))?;
                    responded = true;
                    eprintln!("[+]    Sent 0x0108 response");
                } else if cmd == CMD_READY {
                    eprintln!("[+]    Received 0x0110 'Ready to Stream'!");
                    ready = true;
                    break;
                }
            }
            if ready {
                break;
            }
        }
        s_auth.set_read_timeout(Some(Duration::from_secs(5))).ok();
        if !ready {
            eprintln!("[*]    No explicit ready signal, continuing...");
        }

        // ── 4. Open video channels ───────────────────────────────────────
        eprintln!("[*] 2. Opening video channels on port {PORT_VIDEO}...");
        std::thread::sleep(Duration::from_millis(300));
        let mut s_video = open(proj_ip, PORT_VIDEO)?;
        s_video.write_all(&video_init(my_ip, 0))?;
        eprintln!("[+]    Video channel OPEN (byte28=0x00)");
        let mut s_aux = open(proj_ip, PORT_VIDEO)?;
        s_aux.write_all(&video_init(my_ip, 1))?;
        eprintln!("[+]    Aux channel OPEN (byte28=0x01)");

        // ── 5. Wait for 0x0016 ───────────────────────────────────────────
        eprintln!("[*] 3. Waiting for 0x0016 streaming signal...");
        let data = recv_one(&mut s_auth, Duration::from_secs(10));
        match eemp_messages(&data).first() {
            Some((cmd, _)) => eprintln!("[+]    Received cmd=0x{cmd:04x} ({} bytes)", data.len()),
            None => eprintln!("[*]    No 0x0016 received, continuing..."),
        }

        // ── 6. Warmup buffers ────────────────────────────────────────────
        eprintln!("[*] 4. Sending warmup buffers...");
        for size in [7276u32, 2646, 1764] {
            s_aux.write_all(&aux_header(size))?;
            s_aux.write_all(&vec![0u8; size as usize])?;
            std::thread::sleep(Duration::from_millis(50));
        }
        std::thread::sleep(Duration::from_millis(500));

        eprintln!("\n[+] BINGO! Ready for video stream!");
        Ok(EpsonClient { my_ip, proj_ip, s_auth, s_video, s_aux })
    }

    /// Says goodbye (cmd 0x0104) like the Windows client does on stop, so the
    /// projector frees the session at once instead of waiting for a timeout.
    pub fn disconnect(&mut self) {
        if self.s_auth.write_all(&eemp_header(self.my_ip, CMD_DISCONNECT, 0)).is_ok() {
            let _ = recv_one(&mut self.s_auth, Duration::from_secs(1)); // 0x0105
        }
    }
}

/// Send one 100 ms slice of silent audio on the aux channel. The Windows client
/// alternates 2646- and 1764-byte zero buffers every 50 ms; call this every 100 ms.
pub fn send_keepalive(s_aux: &mut TcpStream) -> io::Result<()> {
    s_aux.write_all(&aux_header(2646))?;
    s_aux.write_all(&[0u8; 2646])?;
    s_aux.write_all(&aux_header(1764))?;
    s_aux.write_all(&[0u8; 1764])?;
    Ok(())
}

/// Answer pending projector heartbeat queries (0x010E) on the control channel.
/// Unanswered, the projector resets the session after ~50 seconds.
///
/// Uses a non-blocking read rather than a tiny read timeout: on Windows a timed-out
/// socket read leaves the socket in an undefined state. Returns an error when the
/// projector has closed the control channel.
pub fn drain_auth(s_auth: &mut TcpStream, my_ip: Ipv4Addr) -> io::Result<()> {
    let mut buf = [0u8; 4096];
    s_auth.set_nonblocking(true)?;
    let r = s_auth.read(&mut buf);
    s_auth.set_nonblocking(false)?;
    let n = match r {
        Ok(0) => return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "projector closed the control channel")),
        Ok(n) => n,
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
        Err(e) => return Err(e),
    };
    for (cmd, _) in eemp_messages(&buf[..n]) {
        if cmd == CMD_STATUS_QUERY {
            s_auth.write_all(&response_0x0108(my_ip))?;
        }
    }
    Ok(())
}

/// Send video frame data.
pub fn send_frame(stream: &mut TcpStream, data: &[u8]) -> io::Result<()> {
    stream.write_all(data)
}

// ─── Custom EPRD Frame Builder ───────────────────────────────────────────────
// Builds frames from scratch — no template needed, no COM padding, no gray boxes.

/// 46-byte display config, captured from the Windows iProjection session and
/// verified byte-for-byte against `windows_perfect_stream.bin`. Sent once, as
/// the very first EPRD block, before any JPEG frame.
const META_DISPLAY_CONFIG: [u8; 46] = [
    0xcc, 0x00, 0x00, 0x00, 0x04, 0x00, 0x03, 0x00,
    0x20, 0x20, 0x00, 0x01, 0xff, 0x00, 0xff, 0x00,
    0xff, 0x00, 0x10, 0x08, 0x00, 0x00, 0x00, 0x00,
    0x06, 0x40, 0x03, 0x84, 0x00, 0x00, 0x00, 0x60,
    0x04, 0x00, 0x02, 0x40, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// The Windows client's keyframe layout at 1024x768, from `windows_perfect_stream.bin`:
/// `(x, y, w, h, ts, jpeg_budget)`. `ts` is copied verbatim (proven on hardware).
/// `jpeg_budget` is the JPEG size Windows used for that tile; we aim for it so the
/// bitrate stays where the old template kept it, but never cut a JPEG to fit.
pub const KEYFRAME_TILES: [(u16, u16, u16, u16, u32, usize); 4] = [
    (0, 0, 624, 416, 2429847810, 34004),
    (624, 0, 400, 416, 2430131968, 12248),
    (0, 416, 624, 352, 2428173568, 16058),
    (624, 416, 400, 352, 2429283840, 14155),
];

/// One JPEG-encoded region of a video frame, with its placement and timestamp.
///
/// Mirrors the 16-byte region descriptor in the EasyMP video stream:
/// `x, y, w, h` (big-endian `u16`) + `flags` (always `0x0000_0007`) + `ts`
/// (big-endian `u32` timestamp), immediately followed by the raw JPEG bytes.
pub struct VideoTile<'a> {
    pub jpeg: &'a [u8],
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    pub ts: u32,
}

/// Build a complete EPRD video frame from JPEG tiles, entirely from scratch.
///
/// This reproduces the exact wire format Windows iProjection emits, so it works
/// at whatever resolution and tile layout the projector negotiates — unlike the
/// frozen `windows_perfect_stream.bin` template, which is welded to a single
/// 1024x768 / 4-tile geometry.
///
/// * `frame_type` — `4` = full keyframe (tiles cover the whole screen); `3` or
///   `1` = partial delta frames carrying only changed regions.
/// * `first_frame` — when true, prepends the one-time META display-config block.
///
/// Note the endianness split confirmed from the capture: the META block's size
/// field is little-endian, the JPEG block's size field is big-endian.
pub fn build_video_frame(
    my_ip: Ipv4Addr,
    tiles: &[VideoTile],
    frame_type: u32,
    first_frame: bool,
) -> Vec<u8> {
    let ip = my_ip.octets();
    let mut buf = Vec::with_capacity(16384);

    // First frame: prepend the META EPRD block (size is little-endian here).
    if first_frame {
        buf.extend_from_slice(b"EPRD0600");
        buf.extend_from_slice(&ip);
        buf.extend_from_slice(&0u32.to_le_bytes()); // msg_id
        buf.extend_from_slice(&(META_DISPLAY_CONFIG.len() as u32).to_le_bytes());
        buf.extend_from_slice(&META_DISPLAY_CONFIG);
    }

    // JPEG payload: frame_type(4) + N × (16-byte region descriptor + jpeg data).
    let mut payload = Vec::new();
    payload.extend_from_slice(&frame_type.to_be_bytes());

    for t in tiles {
        payload.extend_from_slice(&t.x.to_be_bytes());
        payload.extend_from_slice(&t.y.to_be_bytes());
        payload.extend_from_slice(&t.w.to_be_bytes());
        payload.extend_from_slice(&t.h.to_be_bytes());
        payload.extend_from_slice(&0x0000_0007u32.to_be_bytes()); // flags
        payload.extend_from_slice(&t.ts.to_be_bytes());
        payload.extend_from_slice(t.jpeg);
    }

    // JPEG EPRD header (size is big-endian here).
    buf.extend_from_slice(b"EPRD0600");
    buf.extend_from_slice(&ip);
    buf.extend_from_slice(&0u32.to_be_bytes()); // msg_id
    buf.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    buf.extend_from_slice(&payload);

    buf
}

/// Linux TCP keepalive
#[cfg(target_os = "linux")]
fn enable_tcp_keepalive(stream: &TcpStream) {
    use libc::{setsockopt, SOL_SOCKET, SO_KEEPALIVE, IPPROTO_TCP};
    let fd = stream.as_raw_fd();
    // SAFETY: `fd` is a valid open socket owned by `stream`, which outlives this
    // call. Each option pointer refers to a live stack `c_int` and the passed
    // length is exactly `size_of::<c_int>()`, so setsockopt reads in bounds.
    unsafe {
        let val: libc::c_int = 1;
        setsockopt(fd, SOL_SOCKET, SO_KEEPALIVE,
            &val as *const _ as *const libc::c_void, std::mem::size_of::<libc::c_int>() as u32);
        let idle: libc::c_int = 10;
        setsockopt(fd, IPPROTO_TCP, libc::TCP_KEEPIDLE,
            &idle as *const _ as *const libc::c_void, std::mem::size_of::<libc::c_int>() as u32);
        let interval: libc::c_int = 5;
        setsockopt(fd, IPPROTO_TCP, libc::TCP_KEEPINTVL,
            &interval as *const _ as *const libc::c_void, std::mem::size_of::<libc::c_int>() as u32);
        let count: libc::c_int = 3;
        setsockopt(fd, IPPROTO_TCP, libc::TCP_KEEPCNT,
            &count as *const _ as *const libc::c_void, std::mem::size_of::<libc::c_int>() as u32);
    }
}

/// macOS TCP keepalive — uses TCP_KEEPALIVE instead of TCP_KEEPIDLE
#[cfg(target_os = "macos")]
fn enable_tcp_keepalive(stream: &TcpStream) {
    use libc::{setsockopt, SOL_SOCKET, SO_KEEPALIVE, IPPROTO_TCP};
    let fd = stream.as_raw_fd();
    // SAFETY: `fd` is a valid open socket owned by `stream`, which outlives this
    // call. Each option pointer refers to a live stack `c_int` and the passed
    // length is exactly `size_of::<c_int>()`, so setsockopt reads in bounds.
    unsafe {
        let val: libc::c_int = 1;
        setsockopt(fd, SOL_SOCKET, SO_KEEPALIVE,
            &val as *const _ as *const libc::c_void, std::mem::size_of::<libc::c_int>() as u32);
        // macOS uses TCP_KEEPALIVE = 0x10 instead of TCP_KEEPIDLE
        let idle: libc::c_int = 10;
        setsockopt(fd, IPPROTO_TCP, 0x10, // TCP_KEEPALIVE
            &idle as *const _ as *const libc::c_void, std::mem::size_of::<libc::c_int>() as u32);
        let interval: libc::c_int = 5;
        setsockopt(fd, IPPROTO_TCP, libc::TCP_KEEPINTVL,
            &interval as *const _ as *const libc::c_void, std::mem::size_of::<libc::c_int>() as u32);
        let count: libc::c_int = 3;
        setsockopt(fd, IPPROTO_TCP, libc::TCP_KEEPCNT,
            &count as *const _ as *const libc::c_void, std::mem::size_of::<libc::c_int>() as u32);
    }
}

/// Windows TCP keepalive
#[cfg(target_os = "windows")]
fn enable_tcp_keepalive(stream: &TcpStream) {
    use winapi::um::winsock2::setsockopt;
    use winapi::shared::ws2def::{SOL_SOCKET, SO_KEEPALIVE};
    let sock = stream.as_raw_socket() as usize;
    // SAFETY: `sock` is a valid open socket owned by `stream`, which outlives this
    // call. The option pointer refers to a live stack `i32` and the passed length
    // is exactly `size_of::<i32>()`, so setsockopt reads in bounds.
    unsafe {
        let val: i32 = 1;
        setsockopt(sock, SOL_SOCKET as i32, SO_KEEPALIVE as i32,
            &val as *const _ as *const i8, std::mem::size_of::<i32>() as i32);
    }
}
