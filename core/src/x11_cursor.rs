//! draw mouse pointer into x11 frames (xshm grab never has it)

use xcb::{x, xfixes, Connection};

// x11 link for pointer place + look; none if no xfixes
pub struct CursorSource {
    conn: Connection,
}

impl CursorSource {
    // connect display, enable xfixes
    pub fn new() -> Option<Self> {
        let (conn, _) = Connection::connect_with_extensions(None, &[xcb::Extension::XFixes], &[]).ok()?;
        // XFixes requires a version handshake before any other request.
        let cookie = conn.send_request(&xfixes::QueryVersion {
            client_major_version: 5,
            client_minor_version: 0,
        });
        conn.wait_for_reply(cookie).ok()?;
        Some(CursorSource { conn })
    }

    // blend pointer into downscaled rgb frame; no cursor = no-op
    pub fn draw_into_rgb(&self, dst: &mut [u8], dw: u32, dh: u32, sw: u32, sh: u32) {
        let Some(c) = self.cursor() else { return };
        if sw == 0 || sh == 0 || c.w == 0 || c.h == 0 {
            return;
        }
        // Top-left of the cursor bitmap, in screen pixels.
        let left = c.x - c.xhot as i32;
        let top = c.y - c.yhot as i32;

        for cy in 0..c.h {
            for cx in 0..c.w {
                // ARGB, alpha-premultiplied, as XFixes hands it over.
                let px = c.pixels[(cy * c.w + cx) as usize];
                let a = (px >> 24) & 0xff;
                if a == 0 {
                    continue;
                }
                // Screen position of this cursor pixel, scaled into the frame.
                let sx = left + cx as i32;
                let sy = top + cy as i32;
                if sx < 0 || sy < 0 || sx >= sw as i32 || sy >= sh as i32 {
                    continue;
                }
                let dx = (sx as u64 * dw as u64 / sw as u64) as usize;
                let dy = (sy as u64 * dh as u64 / sh as u64) as usize;
                if dx >= dw as usize || dy >= dh as usize {
                    continue;
                }
                let i = (dy * dw as usize + dx) * 3;
                let src = [(px >> 16) & 0xff, (px >> 8) & 0xff, px & 0xff];
                for (k, s) in src.iter().enumerate() {
                    // src-over with premultiplied source: dst = src + dst*(1-a).
                    let out = *s + (dst[i + k] as u32 * (255 - a)) / 255;
                    dst[i + k] = out.min(255) as u8;
                }
            }
        }
    }

    fn cursor(&self) -> Option<CursorImage> {
        let cookie = self.conn.send_request(&xfixes::GetCursorImage {});
        let r = self.conn.wait_for_reply(cookie).ok()?;
        Some(CursorImage {
            x: r.x() as i32,
            y: r.y() as i32,
            w: r.width() as u32,
            h: r.height() as u32,
            xhot: r.xhot(),
            yhot: r.yhot(),
            pixels: r.cursor_image().to_vec(),
        })
    }
}

struct CursorImage {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    xhot: u16,
    yhot: u16,
    pixels: Vec<u32>,
}

/// Keeps the `x` import used even when only XFixes requests are sent.
#[allow(dead_code)]
type _X = x::Window;

#[cfg(test)]
mod tests {
    // blend math w/o x server: opaque overwrites, transparent leaves frame
    #[test]
    fn premultiplied_blend_is_src_over() {
        let blend = |src: u32, a: u32, dst: u8| -> u8 {
            ((src + (dst as u32 * (255 - a)) / 255).min(255)) as u8
        };
        assert_eq!(blend(255, 255, 10), 255, "opaque source wins");
        assert_eq!(blend(0, 0, 77), 77, "transparent source keeps the frame");
        // Half-transparent white over black is mid grey.
        assert_eq!(blend(128, 128, 0), 128);
    }
}
