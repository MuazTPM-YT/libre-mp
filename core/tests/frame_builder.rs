//! Proves the from-scratch EPRD frame builder reproduces the real Windows
//! iProjection video stream captured in `windows_perfect_stream.bin`.
//!
//! If these pass, our dynamic builder emits byte-identical frames to the
//! proprietary Windows client for the same tiles — the foundation for streaming
//! at arbitrary resolutions instead of the frozen 1024x768 template.

use std::net::Ipv4Addr;
use libremp_core::protocol::{build_video_frame, VideoTile};

/// Locate the capture file relative to the crate (tests run with CWD = `core/`).
fn capture() -> Vec<u8> {
    for p in ["../windows_perfect_stream.bin", "windows_perfect_stream.bin"] {
        if let Ok(b) = std::fs::read(p) {
            return b;
        }
    }
    panic!("windows_perfect_stream.bin not found at workspace root");
}

/// Parse the tiles out of the capture's first full-frame block (block 1).
/// Returns (x, y, w, h, ts, jpeg_bytes) per tile.
fn parse_block1_tiles(buf: &[u8]) -> Vec<(u16, u16, u16, u16, u32, Vec<u8>)> {
    // Block 0 = META (66 bytes). Block 1 JPEG payload starts at 86 (66 + 20 header).
    let ps = 86usize;
    assert_eq!(
        u32::from_be_bytes(buf[ps..ps + 4].try_into().unwrap()),
        4,
        "block 1 should be a full keyframe"
    );

    let mut tiles = Vec::new();
    let mut tp = 4usize;
    while tp + 16 <= (buf.len() - ps) {
        let d = ps + tp;
        let x = u16::from_be_bytes(buf[d..d + 2].try_into().unwrap());
        let y = u16::from_be_bytes(buf[d + 2..d + 4].try_into().unwrap());
        let w = u16::from_be_bytes(buf[d + 4..d + 6].try_into().unwrap());
        let h = u16::from_be_bytes(buf[d + 6..d + 8].try_into().unwrap());
        if w == 0 || h == 0 || w > 2000 {
            break;
        }
        let ts = u32::from_be_bytes(buf[d + 12..d + 16].try_into().unwrap());
        let jstart = d + 16;
        if buf[jstart] != 0xff || buf[jstart + 1] != 0xd8 {
            break;
        }
        let mut e = jstart;
        while !(buf[e] == 0xff && buf[e + 1] == 0xd9) {
            e += 1;
        }
        tiles.push((x, y, w, h, ts, buf[jstart..e + 2].to_vec()));
        tp = (e + 2) - ps;
        if tiles.len() == 4 {
            break; // block 1 has exactly 4 tiles
        }
    }
    tiles
}

#[test]
fn builder_reproduces_capture_first_frame_byte_for_byte() {
    let buf = capture();
    // The capture's full-frame block (block 1) spans [66, 76619).
    let expected = &buf[66..76619];

    let owned = parse_block1_tiles(&buf);
    assert_eq!(owned.len(), 4, "expected 4 tiles in block 1");

    let tiles: Vec<VideoTile> = owned
        .iter()
        .map(|(x, y, w, h, ts, j)| VideoTile { jpeg: j, x: *x, y: *y, w: *w, h: *h, ts: *ts })
        .collect();

    // Client IP in the capture's EPRD header is 192.168.88.2.
    let built = build_video_frame(Ipv4Addr::new(192, 168, 88, 2), &tiles, false);

    assert_eq!(built.len(), expected.len(), "frame length mismatch");
    assert!(built == expected, "frame is not byte-identical to the Windows capture");
}

#[test]
fn first_frame_meta_matches_capture() {
    let buf = capture();
    let dummy = [0xffu8, 0xd8, 0x00, 0xff, 0xd9];
    let tiles = [VideoTile { jpeg: &dummy, x: 0, y: 0, w: 8, h: 8, ts: 0 }];
    let built = build_video_frame(Ipv4Addr::new(192, 168, 88, 2), &tiles, true);
    // First 66 bytes = EPRD header (20) + META (46), independent of the tiles.
    assert_eq!(&built[..66], &buf[..66], "META display-config block mismatch vs capture");
}

// x, y, w, h, flags, ts, jpeg
type Tile = (u16, u16, u16, u16, u32, u32, Vec<u8>);

// split one jpeg block payload into tiles: count, then 16-byte descriptor + jpeg each
fn parse_tiles(p: &[u8]) -> (u32, Vec<Tile>) {
    let count = u32::from_be_bytes(p[..4].try_into().unwrap());
    let be16 = |i: usize| u16::from_be_bytes(p[i..i + 2].try_into().unwrap());
    let be32 = |i: usize| u32::from_be_bytes(p[i..i + 4].try_into().unwrap());
    let mut tiles = Vec::new();
    let mut at = 4;
    while at < p.len() {
        let j = at + 16;
        // jpeg end = ffd9 followed by payload end or next tile's ffd8
        let mut e = j + 2;
        loop {
            e += p[e..].windows(2).position(|w| w == [0xff, 0xd9]).unwrap() + 2;
            if e == p.len() || (e + 18 <= p.len() && p[e + 16..e + 18] == [0xff, 0xd8]) {
                break;
            }
        }
        tiles.push((be16(at), be16(at + 2), be16(at + 4), be16(at + 6), be32(at + 8), be32(at + 12), p[j..e].to_vec()));
        at = e;
    }
    (count, tiles)
}

// every windows frame, whole and partial, rebuilt byte for byte; count field = tile count
#[test]
fn builder_reproduces_every_captured_frame() {
    let buf = capture();
    let mut at = 20 + 46; // skip first META block
    let mut blocks = 0;
    let mut partial = 0;
    while at < buf.len() {
        assert_eq!(&buf[at..at + 8], b"EPRD0600");
        let size = u32::from_be_bytes(buf[at + 16..at + 20].try_into().unwrap()) as usize;
        let block = &buf[at..at + 20 + size];
        let (count, owned) = parse_tiles(&block[20..]);
        assert_eq!(count as usize, owned.len(), "count field is the tile count");
        assert!(owned.iter().all(|t| t.4 == 7), "descriptor flags always 7");
        let tiles: Vec<VideoTile> = owned
            .iter()
            .map(|(x, y, w, h, _, ts, j)| VideoTile { jpeg: j, x: *x, y: *y, w: *w, h: *h, ts: *ts })
            .collect();
        let covered: u32 = owned.iter().map(|t| t.2 as u32 * t.3 as u32).sum();
        if covered < 1024 * 768 {
            partial += 1;
        }
        assert!(build_video_frame(Ipv4Addr::new(192, 168, 88, 2), &tiles, false) == block, "block {blocks} differs");
        at += 20 + size;
        blocks += 1;
    }
    assert_eq!(blocks, 105);
    assert!(partial > 90, "windows sends mostly partial frames, got {partial}");
}
