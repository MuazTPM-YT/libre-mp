//! libremp-core: epson protocol, capture, cast loop, wi-fi, qr, saved projectors

pub mod hex;
pub mod wifi;
pub mod capture;
#[cfg(target_os = "linux")]
pub mod screencast;
#[cfg(target_os = "linux")]
pub mod x11_cursor;
pub mod protocol;
pub mod config;
pub mod qr;
pub mod session;

// stream width epson wants
pub const STREAM_W: u32 = 1024;
// stream height epson wants
pub const STREAM_H: u32 = 768;
// jpeg quality before per-tile step-down
pub const JPEG_QUALITY: i32 = 95;
