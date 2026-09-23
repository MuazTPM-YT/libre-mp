//! One cast: grab screen, talk to projector, send only changed parts, retry till told stop.

use std::io::{self, Write};
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::capture::{self, FrameGrabber};
use crate::protocol::{self, VideoTile, KEYFRAME_TILES};
use crate::{STREAM_H, STREAM_W};

const TARGET_FPS: u64 = 24;
// silent audio slice cadence on aux channel, same as windows client
const AUDIO_SLICE: Duration = Duration::from_millis(100);
// whole picture resent this often, so any lost part heals fast
const FULL_REFRESH: Duration = Duration::from_secs(1);
// change detection grid, 16-aligned for 4:2:0 jpeg
const CELL: usize = 32;
// biggest tile windows client ever sends
const MAX_TILE_W: u16 = 624;
const MAX_TILE_H: u16 = 416;
// parts merged down to this many; past NOISE parts or this much area, whole picture is cheaper
const MAX_RECTS: usize = 8;
const NOISE_RECTS: usize = 64;
const MAX_PARTIAL_AREA: f32 = 0.6;

// what to cast to
#[derive(Debug, Clone, Default)]
pub struct CastOptions {
    pub ssid: String,
    pub password: String,
    pub projector_ip: Option<Ipv4Addr>,
    pub keyword: Option<String>,
    pub give_up_after: Option<u32>,
    pub full_frames_only: bool,
}

// cast progress, told to ui
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum CastEvent {
    Sharing,
    Connecting { attempt: u32 },
    Casting { projector: String },
    Reconnecting { reason: String },
}

// why cast stopped by itself
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailKind {
    Capture,
    Rejected,
    Unreachable,
}

// cast failure, words fit for a person
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CastError {
    pub kind: FailKind,
    pub message: String,
}

impl std::fmt::Display for CastError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

// one screen area in stream pixels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

// pick screen grabber, then cast till stop flag drops
pub fn run(opts: &CastOptions, running: &AtomicBool, on_event: &mut dyn FnMut(CastEvent)) -> Result<(), CastError> {
    on_event(CastEvent::Sharing);
    let mut grabber = capture::detect_grabber().map_err(|message| CastError { kind: FailKind::Capture, message })?;
    run_with(opts, grabber.as_mut(), running, on_event)
}

// cast with given grabber. reconnect on drop, give up after n failed connects
pub fn run_with(
    opts: &CastOptions,
    grabber: &mut dyn FrameGrabber,
    running: &AtomicBool,
    on_event: &mut dyn FnMut(CastEvent),
) -> Result<(), CastError> {
    let full_only = opts.full_frames_only || std::env::var_os("LIBREMP_FULL_FRAMES").is_some_and(|v| v == "1");
    let mut failures = 0u32;
    while running.load(Ordering::Relaxed) {
        on_event(CastEvent::Connecting { attempt: failures + 1 });
        let mut client =
            match protocol::EpsonClient::connect(&opts.password, &opts.ssid, opts.projector_ip, opts.keyword.as_deref()) {
                Ok(c) => c,
                Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                    eprintln!("[-] Connection refused: {e}");
                    return Err(CastError {
                        kind: FailKind::Rejected,
                        message: "The projector refused the connection. If it shows a 4-digit keyword, enter it. \
                                  Another device may also be presenting."
                            .into(),
                    });
                }
                Err(e) => {
                    eprintln!("[-] Connection failed: {e}");
                    failures += 1;
                    if opts.give_up_after.is_some_and(|n| failures >= n) {
                        eprintln!("[-] Giving up after {failures} failed attempts.");
                        return Err(CastError {
                            kind: FailKind::Unreachable,
                            message: "The projector did not answer. Check that it is on and on this network, then try again."
                                .into(),
                        });
                    }
                    eprintln!("[*] Retrying in 3s...");
                    nap(Duration::from_secs(3), running);
                    continue;
                }
            };
        failures = 0;
        on_event(CastEvent::Casting { projector: client.name.clone() });
        let reason = stream(&mut client, grabber, running, full_only);
        if !running.load(Ordering::Relaxed) {
            eprintln!("\n[*] Stop requested, disconnecting...");
            client.disconnect();
            break;
        }
        eprintln!("[-] Stream ended: {reason}");
        on_event(CastEvent::Reconnecting { reason });
        nap(Duration::from_secs(2), running);
    }
    Ok(())
}

// sleep in small steps so stop is quick
fn nap(total: Duration, running: &AtomicBool) {
    let end = Instant::now() + total;
    while running.load(Ordering::Relaxed) && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(50));
    }
}

// what to send for one grabbed screen
enum Plan {
    Skip,
    Full,
    Partial(Vec<Rect>),
}

// keep session alive, grab, send whole or changed parts, hold fps
fn stream(client: &mut protocol::EpsonClient, grabber: &mut dyn FrameGrabber, running: &AtomicBool, full_only: bool) -> String {
    let my_ip = client.my_ip;
    let frame_budget = Duration::from_micros(1_000_000 / TARGET_FPS);
    let mut last_audio = Instant::now();
    let mut last_heartbeat = Instant::now();
    let mut last_full = Instant::now();
    let mut prev: Option<Vec<u8>> = None;
    let mut sent = 0u64;

    while running.load(Ordering::Relaxed) {
        let t0 = Instant::now();

        // upkeep first, so session lives even while capture waits
        if let Err(e) = protocol::drain_auth(&mut client.s_auth, my_ip) {
            return format!("Control channel: {e}");
        }
        if last_heartbeat.elapsed() > Duration::from_secs(30) {
            let _ = client.s_auth.write_all(&protocol::response_0x0108(my_ip));
            last_heartbeat = Instant::now();
        }
        if last_audio.elapsed() > Duration::from_secs(1) {
            last_audio = Instant::now() - AUDIO_SLICE;
        }
        while last_audio.elapsed() >= AUDIO_SLICE {
            if let Err(e) = protocol::send_keepalive(&mut client.s_aux) {
                return format!("Keepalive: {e}");
            }
            last_audio += AUDIO_SLICE;
        }

        let Some(screen) = grabber.grab() else {
            std::thread::sleep(Duration::from_millis(20));
            continue;
        };
        let t_capture = t0.elapsed();

        let plan = match &prev {
            Some(p) if !full_only && last_full.elapsed() < FULL_REFRESH => match dirty_rects(p, &screen) {
                Some(r) if r.is_empty() => Plan::Skip,
                Some(r) => Plan::Partial(r),
                None => Plan::Full,
            },
            _ => Plan::Full,
        };

        let frame = match &plan {
            Plan::Skip => None,
            Plan::Full => {
                last_full = Instant::now();
                let jpegs: Vec<Vec<u8>> = KEYFRAME_TILES
                    .iter()
                    .map(|&(x, y, w, h, _, budget)| capture::encode_tile_adaptive(&screen, x, y, w, h, budget))
                    .collect();
                let tiles: Vec<VideoTile> = KEYFRAME_TILES
                    .iter()
                    .zip(&jpegs)
                    .map(|(&(x, y, w, h, ts, _), jpeg)| VideoTile { jpeg, x, y, w, h, ts })
                    .collect();
                Some(protocol::build_video_frame(my_ip, &tiles, true))
            }
            Plan::Partial(rects) => {
                let jpegs: Vec<Vec<u8>> = rects
                    .iter()
                    .map(|r| capture::encode_tile_adaptive(&screen, r.x, r.y, r.w, r.h, byte_budget(r)))
                    .collect();
                let tiles: Vec<VideoTile> = rects
                    .iter()
                    .zip(&jpegs)
                    .map(|(r, jpeg)| VideoTile { jpeg, x: r.x, y: r.y, w: r.w, h: r.h, ts: KEYFRAME_TILES[0].4 })
                    .collect();
                Some(protocol::build_video_frame(my_ip, &tiles, false))
            }
        };
        let t_encode = t0.elapsed();

        if let Some(frame) = &frame {
            if let Err(e) = protocol::send_frame(&mut client.s_video, frame) {
                return format!("{e}");
            }
            sent += 1;
            if sent <= 5 || sent.is_multiple_of(100) {
                let total_ms = t0.elapsed().as_millis().max(1);
                eprintln!(
                    "  Frame {sent}: {} cap={}ms enc={}ms send={}ms ({}KB)",
                    match &plan { Plan::Partial(r) => format!("{} part(s)", r.len()), _ => "full".into() },
                    t_capture.as_millis(),
                    (t_encode - t_capture).as_millis(),
                    total_ms.saturating_sub(t_encode.as_millis()),
                    frame.len() / 1024,
                );
                // all black almost always = os blocked capture
                if screen.iter().take(10_000).all(|&b| b == 0) {
                    eprintln!("\n[!] WARNING: the captured frame is entirely black.");
                    eprintln!("    -> On Wayland, approve the screen-share prompt when it appears.");
                    eprintln!("    -> On macOS, allow Screen Recording in System Settings > Privacy.\n");
                }
            }
        }
        prev = Some(screen);

        let elapsed = t0.elapsed();
        if elapsed < frame_budget {
            std::thread::sleep(frame_budget - elapsed);
        }
    }
    "Stopped".to_string()
}

// jpeg size aim for a part, same bytes-per-pixel as whole-frame tiles
fn byte_budget(r: &Rect) -> usize {
    let full: usize = KEYFRAME_TILES.iter().map(|t| t.5).sum();
    let area = r.w as usize * r.h as usize;
    (full * area / (STREAM_W as usize * STREAM_H as usize)).max(2048)
}

// changed areas between two rgb frames. empty = no change, none = send whole frame
pub fn dirty_rects(prev: &[u8], cur: &[u8]) -> Option<Vec<Rect>> {
    let (w, h) = (STREAM_W as usize, STREAM_H as usize);
    if prev.len() != cur.len() || cur.len() != w * h * 3 {
        return None;
    }
    let (cols, rows) = (w / CELL, h / CELL);
    let row_bytes = CELL * 3;
    let changed = |r: usize, c: usize| {
        (r * CELL..(r + 1) * CELL).any(|y| {
            let at = (y * w + c * CELL) * 3;
            prev[at..at + row_bytes] != cur[at..at + row_bytes]
        })
    };

    // runs per cell row, grown down while next row has same run
    let mut open: Vec<(usize, usize, usize, usize)> = Vec::new(); // c0, c1, r0, r1 (exclusive)
    let mut done: Vec<(usize, usize, usize, usize)> = Vec::new();
    for r in 0..rows {
        let mut runs = Vec::new();
        let mut c = 0;
        while c < cols {
            if changed(r, c) {
                let c0 = c;
                while c < cols && changed(r, c) {
                    c += 1;
                }
                runs.push((c0, c));
            } else {
                c += 1;
            }
        }
        let mut next = Vec::new();
        for (c0, c1) in runs {
            match open.iter().position(|o| o.0 == c0 && o.1 == c1) {
                Some(i) => {
                    let mut o = open.swap_remove(i);
                    o.3 = r + 1;
                    next.push(o);
                }
                None => next.push((c0, c1, r, r + 1)),
            }
        }
        done.append(&mut open);
        open = next;
    }
    done.append(&mut open);

    let px = |v: usize| (v * CELL) as u16;
    let mut rects: Vec<Rect> =
        done.iter().map(|&(c0, c1, r0, r1)| Rect { x: px(c0), y: px(r0), w: px(c1 - c0), h: px(r1 - r0) }).collect();
    if rects.len() > NOISE_RECTS {
        return None;
    }
    // ponytail: greedy o(n^3) merge, fine at <=64 parts; smarter packing if tiles ever matter more
    while rects.len() > MAX_RECTS {
        let mut best = (0, 1, u32::MAX);
        for i in 0..rects.len() {
            for j in i + 1..rects.len() {
                let waste = rect_area(&union(&rects[i], &rects[j])).saturating_sub(rect_area(&rects[i]) + rect_area(&rects[j]));
                if waste < best.2 {
                    best = (i, j, waste);
                }
            }
        }
        let merged = union(&rects[best.0], &rects[best.1]);
        rects.swap_remove(best.1);
        rects[best.0] = merged;
    }
    if rects.iter().map(rect_area).sum::<u32>() as f32 > MAX_PARTIAL_AREA * (w * h) as f32 {
        return None;
    }
    Some(rects.iter().flat_map(split_tile).collect())
}

// pixel count of rect
fn rect_area(r: &Rect) -> u32 {
    r.w as u32 * r.h as u32
}

// smallest rect covering both
fn union(a: &Rect, b: &Rect) -> Rect {
    let (x, y) = (a.x.min(b.x), a.y.min(b.y));
    Rect { x, y, w: (a.x + a.w).max(b.x + b.w) - x, h: (a.y + a.h).max(b.y + b.h) - y }
}

// cut rect into tiles no bigger than windows client sends
fn split_tile(r: &Rect) -> Vec<Rect> {
    let mut out = Vec::new();
    let mut y = r.y;
    while y < r.y + r.h {
        let h = MAX_TILE_H.min(r.y + r.h - y);
        let mut x = r.x;
        while x < r.x + r.w {
            let w = MAX_TILE_W.min(r.x + r.w - x);
            out.push(Rect { x, y, w, h });
            x += w;
        }
        y += h;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank() -> Vec<u8> {
        vec![0u8; (STREAM_W * STREAM_H * 3) as usize]
    }

    fn paint(buf: &mut [u8], x: usize, y: usize, w: usize, h: usize) {
        for yy in y..y + h {
            for xx in x..x + w {
                buf[(yy * STREAM_W as usize + xx) * 3] = 200;
            }
        }
    }

    // same frame, nothing to send
    #[test]
    fn no_change_is_empty() {
        assert_eq!(dirty_rects(&blank(), &blank()), Some(vec![]));
    }

    // small change snaps out to 32px cells
    #[test]
    fn small_change_snaps_to_cells() {
        let mut cur = blank();
        paint(&mut cur, 40, 70, 10, 3);
        assert_eq!(dirty_rects(&blank(), &cur), Some(vec![Rect { x: 32, y: 64, w: 32, h: 32 }]));
    }

    // two far spots stay two parts
    #[test]
    fn two_spots_two_parts() {
        let mut cur = blank();
        paint(&mut cur, 0, 0, 5, 5);
        paint(&mut cur, 1000, 680, 5, 5);
        let r = dirty_rects(&blank(), &cur).unwrap();
        assert_eq!(r.len(), 2);
        assert!(r.contains(&Rect { x: 992, y: 672, w: 32, h: 32 }));
    }

    // tall same-width runs merge into one part
    #[test]
    fn column_merges() {
        let mut cur = blank();
        paint(&mut cur, 100, 100, 50, 200);
        assert_eq!(dirty_rects(&blank(), &cur), Some(vec![Rect { x: 96, y: 96, w: 64, h: 224 }]));
    }

    // big change means whole frame
    #[test]
    fn big_change_is_full() {
        let mut cur = blank();
        paint(&mut cur, 0, 0, 1024, 600);
        assert_eq!(dirty_rects(&blank(), &cur), None);
    }

    // many spots merge down to few parts that still cover every spot
    #[test]
    fn many_spots_merge() {
        let mut cur = blank();
        for i in 0..10 {
            paint(&mut cur, i * 64, 300, 4, 4);
        }
        let r = dirty_rects(&blank(), &cur).unwrap();
        assert_eq!(r.len(), MAX_RECTS);
        assert!((0..10).all(|i| r.iter().any(|p| p.x as usize <= i * 64 && i * 64 < (p.x + p.w) as usize && p.y == 288)));
        assert_eq!(r.iter().map(rect_area).sum::<u32>(), 12 * 32 * 32);
    }

    // far corners stay two small parts, not one giant box
    #[test]
    fn corners_stay_small() {
        let mut cur = blank();
        for i in 0..6 {
            paint(&mut cur, 10 + i * 64, 10, 4, 4);
            paint(&mut cur, 1000 - i * 64, 740, 4, 4);
        }
        let r = dirty_rects(&blank(), &cur).unwrap();
        assert!(r.iter().map(rect_area).sum::<u32>() < 20 * 32 * 32, "{r:?}");
    }

    // screen full of noise sends whole frame
    #[test]
    fn noise_is_full() {
        let mut cur = blank();
        for r in 0..24 {
            for c in (0..32).step_by(2) {
                paint(&mut cur, c * 32, r * 32, 2, 2);
            }
        }
        assert_eq!(dirty_rects(&blank(), &cur), None);
    }

    // wide part cut at windows tile limits, all 16-aligned
    #[test]
    fn split_at_limits() {
        let parts = split_tile(&Rect { x: 0, y: 0, w: 1024, h: 480 });
        assert_eq!(parts.len(), 4);
        assert!(parts.iter().all(|p| p.w <= 624 && p.h <= 416 && p.x % 16 == 0 && p.w % 16 == 0));
        assert_eq!(parts.iter().map(|p| p.w as u32 * p.h as u32).sum::<u32>(), 1024 * 480);
    }

    // wrong size input never panics
    #[test]
    fn bad_size_is_full() {
        assert_eq!(dirty_rects(&[0; 10], &[0; 10]), None);
    }
}
