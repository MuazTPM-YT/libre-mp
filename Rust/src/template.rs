use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::{self, Cursor};

#[derive(Debug, Clone)]
pub struct JpegSlot {
    pub offset: usize,  // absolute byte offset of JPEG data in template
    pub size: usize,     // original JPEG byte length
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

pub struct Template {
    pub buf: Vec<u8>,
    pub slots: Vec<JpegSlot>,
}

impl Template {
    pub fn load(path: &str) -> io::Result<Self> {
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

    pub fn swap(&mut self, idx: usize, padded_jpeg: &[u8]) {
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

pub fn pad_jpeg(jpeg: &[u8], target_size: usize) -> Vec<u8> {
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
