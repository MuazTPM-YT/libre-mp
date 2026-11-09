use byteorder::{LittleEndian, WriteBytesExt};
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;
#[cfg(target_os = "macos")]
use std::os::unix::io::AsRawFd;
#[cfg(target_os = "windows")]
use std::os::windows::io::AsRawSocket;

#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::process::Command;

use crate::hex;

// epson simple ap address, tried last
const DEFAULT_PROJECTOR_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 88, 1);
const PORT_CONTROL: u16 = 3620;
const PORT_VIDEO: u16 = 3621;
// bounded connect: wrong address fail in seconds, not os ~2 min
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

// eemp commands seen in windows capture
const CMD_REGISTER: u32 = 0x0002;
const CMD_REGISTER_INFO: u32 = 0x0003;
const CMD_REGISTER_MAC: u32 = 0x0015;
const CMD_AUTH_OK: u32 = 0x0102;
const CMD_DISCONNECT: u32 = 0x0104;
const CMD_DISCONNECT_OK: u32 = 0x0105;
const CMD_STATUS_QUERY: u32 = 0x010E;
const CMD_READY: u32 = 0x0110;
const CMD_STREAM_GO: u32 = 0x0016;
// bare-header keep-alive, seen in iProjection 4.10 CKeepAlive::Exec
const CMD_KEEPALIVE: u32 = 0x000A;
// iProjection sends it every 5 s when projector has two-way keep-alive
const KEEPALIVE_EVERY: Duration = Duration::from_secs(5);

// every ipv4 default gateway, route-table order. projector often is one
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
        // `route print -4` rows: numeric, same in every locale
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

// where to look: given ip, then gateways, then epson default
fn projector_candidates(override_ip: Option<Ipv4Addr>) -> Vec<Ipv4Addr> {
    let mut all: Vec<Ipv4Addr> = override_ip.into_iter().chain(default_gateways()).collect();
    all.push(DEFAULT_PROJECTOR_IP);
    let mut seen = std::collections::HashSet::new();
    all.retain(|ip| seen.insert(*ip));
    all
}

// tcp connect: no nagle, keepalive, bounded
fn open(ip: Ipv4Addr, port: u16) -> io::Result<TcpStream> {
    let s = TcpStream::connect_timeout(&SocketAddr::from((ip, port)), CONNECT_TIMEOUT)?;
    s.set_nodelay(true)?;
    enable_tcp_keepalive(&s);
    Ok(s)
}

// one recv, whatever is there (max 4096)
fn recv_one(stream: &mut TcpStream, timeout: Duration) -> Vec<u8> {
    stream.set_read_timeout(Some(timeout)).ok();
    let mut buf = vec![0u8; 4096];
    match stream.read(&mut buf) {
        Ok(n) => buf[..n].to_vec(),
        Err(_) => Vec::new(),
    }
}

// biggest eemp payload we hold; real ones are < 1 KB, so bigger = junk length
const MAX_EEMP_PAYLOAD: usize = 64 * 1024;

// control bytes cut into whole eemp messages. tcp splits and joins messages, so partial tail waits for next read
#[derive(Debug, Default)]
pub struct EempBuf {
    buf: Vec<u8>,
}

impl EempBuf {
    // add bytes as they came off the socket
    pub fn push(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    // every whole (cmd, payload) so far. junk before a magic is skipped
    pub fn take(&mut self) -> Vec<(u32, Vec<u8>)> {
        let mut out = Vec::new();
        let mut off = 0;
        loop {
            let Some(at) = self.buf[off..].windows(8).position(|w| w == b"EEMP0100") else {
                // keep last 7 bytes: a magic may be cut in half
                off = self.buf.len().saturating_sub(7).max(off);
                break;
            };
            off += at;
            if off + 20 > self.buf.len() {
                break;
            }
            let cmd = u32::from_le_bytes(self.buf[off + 12..off + 16].try_into().unwrap());
            let len = u32::from_le_bytes(self.buf[off + 16..off + 20].try_into().unwrap()) as usize;
            if len > MAX_EEMP_PAYLOAD {
                off += 1; // bogus length: resync on next magic
                continue;
            }
            if off + 20 + len > self.buf.len() {
                break;
            }
            out.push((cmd, self.buf[off + 20..off + 20 + len].to_vec()));
            off += 20 + len;
        }
        self.buf.drain(..off);
        out
    }
}

// whole eemp messages in one complete buffer
fn eemp_messages(data: &[u8]) -> Vec<(u32, Vec<u8>)> {
    let mut b = EempBuf::default();
    b.push(data);
    b.take()
}

// one blocking read into buffer; false = timeout, closed or error
fn fill(stream: &mut TcpStream, ctl: &mut EempBuf, timeout: Duration) -> bool {
    stream.set_read_timeout(Some(timeout)).ok();
    let mut chunk = [0u8; 4096];
    match stream.read(&mut chunk) {
        Ok(n) if n > 0 => {
            ctl.push(&chunk[..n]);
            true
        }
        _ => false,
    }
}

// answer every heartbeat query (0x010E) in msgs, give back their cmds
fn answer_queries(s_auth: &mut TcpStream, my_ip: Ipv4Addr, msgs: Vec<(u32, Vec<u8>)>) -> io::Result<Vec<u32>> {
    let mut cmds = Vec::with_capacity(msgs.len());
    for (cmd, _) in msgs {
        if cmd == CMD_STATUS_QUERY {
            s_auth.write_all(&response_0x0108(my_ip))?;
        }
        cmds.push(cmd);
    }
    Ok(cmds)
}

// 20-byte eemp header: magic, sender ip, cmd, len (le)
fn eemp_header(my_ip: Ipv4Addr, cmd: u32, payload_len: u32) -> Vec<u8> {
    let mut h = Vec::with_capacity(20);
    h.extend_from_slice(b"EEMP0100");
    h.extend_from_slice(&my_ip.octets());
    h.extend_from_slice(&cmd.to_le_bytes());
    h.extend_from_slice(&payload_len.to_le_bytes());
    h
}

// mac from 12 hex digits, ':' or '-' allowed
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

// registration (0x0002), byte-same as windows
pub fn registration_payload(my_ip: Ipv4Addr) -> Vec<u8> {
    let mut p = eemp_header(my_ip, CMD_REGISTER, 48);
    p.extend_from_slice(&hex::decode("007f0000b0f8ef5314000000").unwrap());
    p.extend_from_slice(&[0u8; 36]);
    p
}

// what projector says about itself
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ProjectorIdentity {
    // name (0x0003)
    pub name: Option<Vec<u8>>,
    // mac (0x0015, else 0x0003) = easymp auth key
    pub mac: Option<[u8; 6]>,
    // what it can do (0x0015)
    pub caps: ProjectorCaps,
}

// projector abilities from 0x0015. layout read from iProjection 4.10, see docs/IPROJECTION_RE.md
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ProjectorCaps {
    // frame size it reports (record 3)
    pub frame: Option<(u16, u16)>,
    // biggest jpeg tile (record 5, encoding 07/0b; windows tiles match it)
    pub max_tile: Option<(u16, u16)>,
    // function flag bytes (record 8)
    pub flags: Vec<u8>,
}

impl ProjectorCaps {
    // 0x0015 payload: 26-byte head, u16 be length, 2 bytes, then u8 type + u8 len records
    pub fn parse(p: &[u8]) -> Self {
        let mut caps = Self::default();
        let Some(len) = p.get(26..28).map(|b| u16::from_be_bytes([b[0], b[1]]) as usize) else { return caps };
        let mut rec = p.get(30..p.len().min(30 + len)).unwrap_or_default();
        // ponytail: stop at type 0; tail after record 10 not understood yet
        while let [kind, n, rest @ ..] = rec {
            let n = *n as usize;
            if *kind == 0 || n > rest.len() {
                break;
            }
            let (data, next) = rest.split_at(n);
            let le = |i: usize| u16::from_le_bytes([data[i], data[i + 1]]);
            match kind {
                3 if n >= 4 => caps.frame = Some((le(0), le(2))),
                // 22-byte entries: enc, sub, align, unit, min, max (u16 le pairs), throughput
                5 => {
                    caps.max_tile = data.as_chunks::<22>().0.iter().find(|e| e[..2] == [0x07, 0x0b]).map(|e| {
                        (u16::from_le_bytes([e[14], e[15]]), u16::from_le_bytes([e[16], e[17]]))
                    })
                }
                8 => caps.flags = data.to_vec(),
                _ => {}
            }
            rec = next;
        }
        caps
    }

    // one flag bit; missing byte = no
    fn flag(&self, byte: usize, bit: u8) -> bool {
        self.flags.get(byte).is_some_and(|b| b >> bit & 1 == 1)
    }

    // projector wants eemp keep-alive both ways
    pub fn two_way_keepalive(&self) -> bool {
        self.flag(8, 0)
    }

    // projector can encrypt (des, aes, aes-ep-ctr); libremp sends plain, like iprojection default
    pub fn encryption(&self) -> (bool, bool, bool) {
        (self.flag(1, 0), self.flag(1, 1), self.flag(1, 2))
    }

    // projector can show a keyword on screen
    pub fn display_keyword(&self) -> bool {
        self.flag(2, 6)
    }
}

// name + mac from registration reply, like windows client
pub fn parse_registration_response(data: &[u8]) -> ProjectorIdentity {
    let nonzero = |b: &[u8]| -> Option<[u8; 6]> {
        let m: [u8; 6] = b.try_into().ok()?;
        (m != [0; 6]).then_some(m)
    };
    let mut id = ProjectorIdentity::default();
    let mut info_mac = None;
    for (cmd, p) in eemp_messages(data) {
        if cmd == CMD_REGISTER_MAC {
            id.caps = ProjectorCaps::parse(&p);
        }
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

// login (0x0101), byte-same as windows w/o keyword. ponytail: keyword spot from rhino pin tool, untested on keyword hardware
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

// reply to projector heartbeat query 0x010E
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

// video port channel init (0 = video, 1 = aux audio)
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

// aux header: 0xC9 + size
fn aux_header(size: u32) -> Vec<u8> {
    let mut h = Vec::with_capacity(5);
    h.push(0xC9);
    h.write_u32::<LittleEndian>(size).unwrap();
    h
}

// ─── Protocol Client ─────────────────────────────────────────────────────────

pub struct EpsonClient {
    pub my_ip: Ipv4Addr,
    pub proj_ip: Ipv4Addr,
    pub name: String,
    pub caps: ProjectorCaps,
    pub s_auth: TcpStream,
    pub s_video: TcpStream,
    pub s_aux: TcpStream,
    // control bytes not yet a whole message
    ctl: EempBuf,
    last_keepalive: Instant,
}

impl EpsonClient {
    // full easymp handshake. password + ssid only fallbacks; ip override tried first
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
        // our ip on interface that reach projector
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
        log_caps(&id.caps);
        std::thread::sleep(Duration::from_millis(100));

        // ── 2. Authentication ────────────────────────────────────────────
        eprintln!("[*]    Authenticating...");
        let mut s_auth = open(proj_ip, PORT_CONTROL)?;
        let mut ctl = EempBuf::default();
        s_auth.write_all(&auth_payload(my_ip, proj_ip, &mac, &name, keyword))?;
        if !fill(&mut s_auth, &mut ctl, Duration::from_secs(5)) {
            eprintln!("[*]    No auth reply (projector may be busy)");
        }
        let mut msgs = ctl.take();
        // 0x0102 byte 51 (payload 30) = 0 means ok
        for (cmd, p) in &msgs {
            eprintln!("[+]    Auth reply cmd=0x{cmd:04x}, {} bytes", p.len() + 20);
            if *cmd == CMD_AUTH_OK && p.len() > 30 && p[30] != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "projector refused the connection (wrong or missing Projector Keyword, or another device is presenting)",
                ));
            }
        }

        // ── 3. Post-auth handshake: answer queries till 0x0110 ──────────
        eprintln!("[*]    Post-auth handshake...");
        let (mut ready, mut stream_go) = (false, false);
        for _ in 0..10 {
            for cmd in answer_queries(&mut s_auth, my_ip, std::mem::take(&mut msgs))? {
                match cmd {
                    CMD_STATUS_QUERY => eprintln!("[+]    Sent 0x0108 response"),
                    CMD_READY => ready = true,
                    CMD_STREAM_GO => stream_go = true,
                    _ => {}
                }
            }
            if ready || !fill(&mut s_auth, &mut ctl, Duration::from_secs(3)) {
                break;
            }
            msgs = ctl.take();
        }
        if ready {
            eprintln!("[+]    Received 0x0110 'Ready to Stream'!");
        } else {
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
        let deadline = Instant::now() + Duration::from_secs(10);
        while !stream_go {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() || !fill(&mut s_auth, &mut ctl, left) {
                eprintln!("[*]    No 0x0016 received, continuing...");
                break;
            }
            for cmd in answer_queries(&mut s_auth, my_ip, ctl.take())? {
                eprintln!("[+]    Received cmd=0x{cmd:04x}");
                stream_go |= cmd == CMD_STREAM_GO;
            }
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
        let name = String::from_utf8_lossy(&name).into_owned();
        Ok(EpsonClient {
            my_ip,
            proj_ip,
            name,
            caps: id.caps,
            s_auth,
            s_video,
            s_aux,
            ctl,
            last_keepalive: Instant::now(),
        })
    }

    // goodbye (0x0104) like windows. wait for 0x0105 before close: projector answers in ~1.2 s, and closing
    // first makes kernel send reset, so projector thinks link died and keeps last picture up
    pub fn disconnect(&mut self) {
        if let Err(e) = self.s_auth.write_all(&eemp_header(self.my_ip, CMD_DISCONNECT, 0)) {
            eprintln!("[-] Goodbye not sent: {e}");
            return;
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut seen: Vec<u32> = Vec::new();
        while !seen.contains(&CMD_DISCONNECT_OK) {
            let left = deadline.saturating_duration_since(Instant::now());
            if !fill(&mut self.s_auth, &mut self.ctl, left.max(Duration::from_millis(1))) {
                break; // timeout or closed
            }
            seen.extend(self.ctl.take().into_iter().map(|(c, _)| c));
        }
        eprintln!("[*] Goodbye reply: {:?}", seen.iter().map(|c| format!("0x{c:04x}")).collect::<Vec<_>>());
    }

    // control upkeep, call often: answer heartbeat queries or projector resets after ~50 s, and send
    // 0x000A keep-alive when projector wants it. nonblocking read: on windows a timed-out read breaks socket
    pub fn service_control(&mut self) -> io::Result<()> {
        let mut chunk = [0u8; 4096];
        self.s_auth.set_nonblocking(true)?;
        let read = loop {
            match self.s_auth.read(&mut chunk) {
                Ok(0) => break Err(io::Error::new(io::ErrorKind::ConnectionAborted, "projector closed the control channel")),
                Ok(n) => self.ctl.push(&chunk[..n]),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break Ok(()),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => break Err(e),
            }
        };
        self.s_auth.set_nonblocking(false)?;
        read?;
        answer_queries(&mut self.s_auth, self.my_ip, self.ctl.take())?;
        if self.caps.two_way_keepalive() && self.last_keepalive.elapsed() >= KEEPALIVE_EVERY {
            self.s_auth.write_all(&eemp_header(self.my_ip, CMD_KEEPALIVE, 0))?;
            self.last_keepalive = Instant::now();
        }
        Ok(())
    }
}

// say what projector reported, so field logs show why tiles or size differ
fn log_caps(c: &ProjectorCaps) {
    let (des, aes, ctr) = c.encryption();
    eprintln!(
        "[+]    Capabilities: frame {:?}, max tile {:?}, two-way keep-alive {}, encryption des={des} aes={aes} aes-ctr={ctr} (LibreMP sends plain), shows keyword {}",
        c.frame,
        c.max_tile,
        c.two_way_keepalive(),
        c.display_keyword(),
    );
    if c.frame.is_some_and(|f| f != (crate::STREAM_W as u16, crate::STREAM_H as u16)) {
        eprintln!(
            "[!]    Projector reports another frame size; LibreMP still sends {}x{}, the only size tested on hardware.",
            crate::STREAM_W,
            crate::STREAM_H
        );
    }
}

// 100ms silent audio on aux (2646 + 1764 zero bytes); call every 100ms
pub fn send_keepalive(s_aux: &mut TcpStream) -> io::Result<()> {
    s_aux.write_all(&aux_header(2646))?;
    s_aux.write_all(&[0u8; 2646])?;
    s_aux.write_all(&aux_header(1764))?;
    s_aux.write_all(&[0u8; 1764])?;
    Ok(())
}

// send video frame bytes
pub fn send_frame(stream: &mut TcpStream, data: &[u8]) -> io::Result<()> {
    stream.write_all(data)
}

// ─── eprd frame builder ─────────────────────────────────────────────────────

// 46-byte display config from windows capture, sent with whole frames. bytes 24..36 = pc 1600x900, picture box (0, 96, 1024, 576).
// RESEARCHLAB ignores the box (shows whole frame); changing these bytes never helped on hardware, so keep windows bytes
const META_DISPLAY_CONFIG: [u8; 46] = [
    0xcc, 0x00, 0x00, 0x00, 0x04, 0x00, 0x03, 0x00,
    0x20, 0x20, 0x00, 0x01, 0xff, 0x00, 0xff, 0x00,
    0xff, 0x00, 0x10, 0x08, 0x00, 0x00, 0x00, 0x00,
    0x06, 0x40, 0x03, 0x84, 0x00, 0x00, 0x00, 0x60,
    0x04, 0x00, 0x02, 0x40, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// one jpeg tile + place (16-byte descriptor on wire)
pub struct VideoTile<'a> {
    pub jpeg: &'a [u8],
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

// jpeg length as projector reads it: 0x90 + 3-byte varint. wrong length = projector drops stream
fn jpeg_len_tag(n: usize) -> [u8; 4] {
    [0x90, 0x80 | (n & 0x7f) as u8, 0x80 | (n >> 7 & 0x7f) as u8, (n >> 14 & 0x7f) as u8]
}

// eprd frame from jpeg tiles, byte-same as windows. count field = tile count
pub fn build_video_frame(my_ip: Ipv4Addr, tiles: &[VideoTile], with_meta: bool) -> Vec<u8> {
    let ip = my_ip.octets();
    let mut buf = Vec::with_capacity(16384);

    // meta block first when asked (size le)
    if with_meta {
        buf.extend_from_slice(b"EPRD0600");
        buf.extend_from_slice(&ip);
        buf.extend_from_slice(&0u32.to_le_bytes()); // msg_id
        buf.extend_from_slice(&(META_DISPLAY_CONFIG.len() as u32).to_le_bytes());
        buf.extend_from_slice(&META_DISPLAY_CONFIG);
    }

    // payload: tile count + per tile 16-byte descriptor + jpeg
    let mut payload = Vec::new();
    payload.extend_from_slice(&(tiles.len() as u32).to_be_bytes());

    for t in tiles {
        payload.extend_from_slice(&t.x.to_be_bytes());
        payload.extend_from_slice(&t.y.to_be_bytes());
        payload.extend_from_slice(&t.w.to_be_bytes());
        payload.extend_from_slice(&t.h.to_be_bytes());
        payload.extend_from_slice(&0x0000_0007u32.to_be_bytes()); // flags
        payload.extend_from_slice(&jpeg_len_tag(t.jpeg.len()));
        payload.extend_from_slice(t.jpeg);
    }

    // jpeg eprd header (size be)
    buf.extend_from_slice(b"EPRD0600");
    buf.extend_from_slice(&ip);
    buf.extend_from_slice(&0u32.to_be_bytes()); // msg_id
    buf.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    buf.extend_from_slice(&payload);

    buf
}

// linux tcp keepalive
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

// macos tcp keepalive (TCP_KEEPALIVE, not KEEPIDLE)
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

// windows tcp keepalive
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
        setsockopt(sock, SOL_SOCKET, SO_KEEPALIVE,
            &val as *const _ as *const i8, std::mem::size_of::<i32>() as i32);
    }
}
