pub const PORT_CONTROL: u16 = 3620;
pub const PORT_VIDEO: u16 = 3621;
pub const STREAM_WIDTH: u32 = 624;
pub const STREAM_HEIGHT: u32 = 416;
pub const PROJECTOR_DISPLAY_WIDTH: u32 = 1600;
pub const PROJECTOR_DISPLAY_HEIGHT: u32 = 900;
pub const JPEG_QUALITY: u8 = 80;

/// Returns the local device's IPv4 address (currently a stubbed implementation).
pub fn get_local_ip() -> String {
    // Basic local IP determination - can be improved later
    "192.168.1.10".to_string()
}
