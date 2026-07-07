//! libremp-core: shared discovery, protocol, capture, and framing logic.

pub mod hex;
pub mod wifi;
pub mod template;
pub mod capture;
pub mod protocol;
pub mod config;
pub mod qr;

/// Streaming frame width negotiated with Epson projectors.
pub const STREAM_W: u32 = 1024;
/// Streaming frame height negotiated with Epson projectors.
pub const STREAM_H: u32 = 768;
/// Baseline JPEG quality before per-tile adaptive downscaling.
pub const JPEG_QUALITY: i32 = 95;
