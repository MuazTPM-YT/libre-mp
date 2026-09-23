//! libremp-core: shared discovery, protocol, capture, and framing logic.

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

/// Streaming frame width negotiated with Epson projectors.
pub const STREAM_W: u32 = 1024;
/// Streaming frame height negotiated with Epson projectors.
pub const STREAM_H: u32 = 768;
/// Baseline JPEG quality before per-tile adaptive downscaling.
pub const JPEG_QUALITY: i32 = 95;
