use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Cursor, Read, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::time::Duration;

use crate::hex;

const PROJECTOR_IP: &str = "192.168.88.1";
const PORT_CONTROL: u16 = 3620;
const PORT_VIDEO: u16 = 3621;
pub const CHUNK_SIZE: usize = 1460;
pub const CHUNK_DELAY: Duration = Duration::from_millis(2);

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

pub fn send_chunked(stream: &mut TcpStream, data: &[u8]) -> io::Result<()> {
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
