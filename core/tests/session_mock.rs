// fake projector on localhost: full handshake, stream, goodbye, and refusal

use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use libremp_core::capture::FrameGrabber;
use libremp_core::session::{run_with, CastEvent, CastOptions, FailKind};

const LOCAL: Ipv4Addr = Ipv4Addr::LOCALHOST;
const NAME: &[u8] = b"MOCKPROJ";
const MAC: [u8; 6] = [0x02, 0, 0, 0, 0, 0x01];

// eemp message: magic, sender ip, cmd, len, payload
fn eemp(cmd: u32, payload: &[u8]) -> Vec<u8> {
    let mut m = b"EEMP0100".to_vec();
    m.extend_from_slice(&[127, 0, 0, 1]);
    m.extend_from_slice(&cmd.to_le_bytes());
    m.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    m.extend_from_slice(payload);
    m
}

// read till buffer holds eemp message with this cmd
fn wait_cmd(s: &mut TcpStream, cmd: u32) -> bool {
    let want = cmd.to_le_bytes();
    let mut seen = Vec::new();
    let mut buf = [0u8; 4096];
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    loop {
        if seen.windows(16).any(|w| &w[..8] == b"EEMP0100" && w[12..16] == want) {
            return true;
        }
        match s.read(&mut buf) {
            Ok(0) | Err(_) => return false,
            Ok(n) => seen.extend_from_slice(&buf[..n]),
        }
    }
}

// what fake projector saw
#[derive(Default)]
struct Seen {
    video: Vec<u8>,
    goodbye: bool,
}

// serve one session. status 0 = accept login, else refuse
fn fake_projector(status: u8) -> Arc<Mutex<Seen>> {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let control = TcpListener::bind((LOCAL, 3620)).expect("port 3620 free");
    let video = TcpListener::bind((LOCAL, 3621)).expect("port 3621 free");

    let s = seen.clone();
    thread::spawn(move || {
        // registration: name + mac, then close
        let (mut reg, _) = control.accept().unwrap();
        wait_cmd(&mut reg, 0x0002);
        let mut info = vec![1, 0, 0, 0];
        info.extend_from_slice(&{
            let mut n = [0u8; 32];
            n[..NAME.len()].copy_from_slice(NAME);
            n
        });
        info.extend_from_slice(&[0u8; 12]);
        info.extend_from_slice(&MAC);
        reg.write_all(&eemp(0x0003, &info)).unwrap();
        drop(reg);

        // auth: status byte 30, then status query, ready, stream go
        let (mut auth, _) = control.accept().unwrap();
        auth.set_nodelay(true).unwrap();
        wait_cmd(&mut auth, 0x0101);
        let mut reply = vec![0u8; 40];
        reply[30] = status;
        auth.write_all(&eemp(0x0102, &reply)).unwrap();
        if status != 0 {
            return;
        }
        thread::sleep(Duration::from_millis(300));
        auth.write_all(&eemp(0x010E, &[])).unwrap();
        assert!(wait_cmd(&mut auth, 0x0108), "client answers status query");
        auth.write_all(&eemp(0x0110, &[])).unwrap();
        thread::sleep(Duration::from_millis(800));
        auth.write_all(&eemp(0x0016, &[])).unwrap();
        if wait_cmd(&mut auth, 0x0104) {
            s.lock().unwrap().goodbye = true;
            let _ = auth.write_all(&eemp(0x0105, &[]));
        }
    });

    let s = seen.clone();
    thread::spawn(move || {
        for _ in 0..2 {
            let Ok((mut c, _)) = video.accept() else { return };
            let mut init = [0u8; 36];
            c.read_exact(&mut init).unwrap();
            let is_video = init[28] == 0;
            let s = s.clone();
            thread::spawn(move || {
                let mut buf = [0u8; 65536];
                while let Ok(n) = c.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    if is_video {
                        s.lock().unwrap().video.extend_from_slice(&buf[..n]);
                    }
                }
            });
        }
    });
    seen
}

// gray screen with small square that moves each grab
struct MovingSquare(usize);

impl FrameGrabber for MovingSquare {
    fn grab(&mut self) -> Option<Vec<u8>> {
        let (w, h) = (1024usize, 768usize);
        let mut rgb = vec![90u8; w * h * 3];
        let x0 = (self.0 * 40) % (w - 20);
        for y in 300..320 {
            for x in x0..x0 + 20 {
                rgb[(y * w + x) * 3] = 250;
            }
        }
        self.0 += 1;
        Some(rgb)
    }
    fn name(&self) -> &'static str {
        "moving square"
    }
}

// eprd blocks as (has meta, tile count, area covered)
fn blocks(stream: &[u8]) -> Vec<(bool, u32, u32)> {
    let mut out = Vec::new();
    let mut at = 0;
    let mut meta = false;
    while at + 20 <= stream.len() {
        assert_eq!(&stream[at..at + 8], b"EPRD0600");
        if stream[at + 20] == 0xcc {
            meta = true;
            at += 20 + u32::from_le_bytes(stream[at + 16..at + 20].try_into().unwrap()) as usize;
            continue;
        }
        let size = u32::from_be_bytes(stream[at + 16..at + 20].try_into().unwrap()) as usize;
        if at + 20 + size > stream.len() {
            break;
        }
        let p = &stream[at + 20..at + 20 + size];
        let count = u32::from_be_bytes(p[..4].try_into().unwrap());
        // walk descriptors: jpeg ends at ffd9 followed by next ffd8 or end
        let (mut i, mut area) = (4usize, 0u32);
        for _ in 0..count {
            let w = u16::from_be_bytes(p[i + 4..i + 6].try_into().unwrap()) as u32;
            let h = u16::from_be_bytes(p[i + 6..i + 8].try_into().unwrap()) as u32;
            area += w * h;
            assert_eq!(&p[i + 16..i + 18], [0xff, 0xd8], "tile holds jpeg");
            let mut e = i + 18;
            loop {
                e += p[e..].windows(2).position(|x| x == [0xff, 0xd9]).unwrap() + 2;
                if e == p.len() || p[e + 16..e + 18] == [0xff, 0xd8] {
                    break;
                }
            }
            i = e;
        }
        assert_eq!(i, p.len(), "tiles fill block exactly");
        out.push((meta, count, area));
        meta = false;
        at += 20 + size;
    }
    out
}

// both scenarios share fixed ports, so run them in order
#[test]
fn fake_projector_session() {
    // accept: stream starts whole, then small parts, then goodbye
    let seen = fake_projector(0);
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let caster = thread::spawn(move || {
        let opts = CastOptions { projector_ip: Some(LOCAL), give_up_after: Some(1), ..Default::default() };
        let mut events = Vec::new();
        let result = run_with(&opts, &mut MovingSquare(0), &r, &mut |e| events.push(e));
        (result, events)
    });
    let start = Instant::now();
    while blocks(&seen.lock().unwrap().video).len() < 12 {
        assert!(start.elapsed() < Duration::from_secs(30), "no video arrived");
        thread::sleep(Duration::from_millis(50));
    }
    running.store(false, Ordering::Relaxed);
    let (result, events) = caster.join().unwrap();
    assert_eq!(result, Ok(()));
    assert!(events.contains(&CastEvent::Casting { projector: "MOCKPROJ".into() }));
    thread::sleep(Duration::from_millis(300));

    let seen = seen.lock().unwrap();
    assert!(seen.goodbye, "client sends 0x0104 on stop");
    let b = blocks(&seen.video);
    assert_eq!(b[0], (true, 4, 1024 * 768), "first frame whole, with meta");
    let partial: Vec<_> = b[1..].iter().filter(|x| x.2 < 1024 * 768).collect();
    assert!(partial.len() >= 8, "later frames send only changed parts: {b:?}");
    assert!(partial.iter().all(|x| !x.0 && x.2 <= 2 * 64 * 32), "parts small, no meta: {b:?}");
    drop(seen);

    // refuse: stops at once with rejected kind
    let _seen = fake_projector(1);
    let running = AtomicBool::new(true);
    let opts = CastOptions { projector_ip: Some(LOCAL), give_up_after: Some(3), ..Default::default() };
    let err = run_with(&opts, &mut MovingSquare(0), &running, &mut |_| {}).unwrap_err();
    assert_eq!(err.kind, FailKind::Rejected);
}
