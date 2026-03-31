use std::net::Ipv4Addr;
use std::time::{SystemTime, UNIX_EPOCH};
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};

pub fn get_hex_ip_bytes(ip: &str) -> Vec<u8> {
    let parsed: Ipv4Addr = ip.parse().unwrap_or(Ipv4Addr::new(0, 0, 0, 0));
    parsed.octets().to_vec()
}

pub fn get_hex_ip_reversed_bytes(ip: &str) -> Vec<u8> {
    let mut parsed: Vec<u8> = get_hex_ip_bytes(ip);
    parsed.reverse();
    parsed
}

pub fn get_registration_payload(my_ip: &str) -> Vec<u8> {
    let mut raw = hex::decode(
        "45454d5030313030000000000200000030000000007f0000b0f8ef5314000000\
         0000000000000000000000000000000000000000000000000000000000000000"
    ).unwrap();
    let ip_bytes = get_hex_ip_bytes(my_ip);
    raw[8..12].copy_from_slice(&ip_bytes);
    raw
}

pub fn get_auth_payload_full(my_ip: &str, proj_ip: &str, my_mac: &str, ssid: &str) -> Vec<u8> {
    let hex_ip_str = hex::encode(get_hex_ip_bytes(my_ip));
    let hex_proj_str = hex::encode(get_hex_ip_bytes(proj_ip));
    
    // Extract projector name from SSID prefix (e.g. "RESEARCHLAB-xxx" → "RESEARCHLAB")
    let proj_name = ssid.split('-').next().unwrap_or(ssid);
    let name_hex = hex::encode(proj_name.as_bytes());
    // Pad to 32 bytes (64 hex chars)
    let name_hex_padded = format!("{:0<64}", name_hex);
    // Length descriptor as 4-byte LE hex
    let name_len = proj_name.len() as u32;
    let len_hex = hex::encode(name_len.to_le_bytes());
    
    let template = format!(
        "45454d5030313030{}01010000f40000000101000000380f000000000000\
         ffffff0000000000020f0b0004000320200001ff00ff00ff00000810000000010e0000\
         {}00000000000000000000000000000000\
         {}a600000005000000380000000200000004000000\
         {}0c0000000400000000000000010000000400000050004300\
         {}\
         04000000000000001c00000000000000040000003600000001000000030000002a000000\
         {}{}{}\
         0f00000004000000320000000d000000040000000200000026000000080000000010000000100000",
        hex_ip_str, my_mac, hex_proj_str, hex_ip_str,
        len_hex,
        my_mac, hex_proj_str, name_hex_padded
    );
    hex::decode(&template).unwrap()
}

pub fn get_video_init_payload_ctrl(my_ip: &str) -> Vec<u8> {
    let mut raw = hex::decode(
        "4550524430363030000000000000000010000000d0000000000000000000000000000000"
    ).unwrap();
    let ip_bytes = get_hex_ip_bytes(my_ip);
    let ip_rev = get_hex_ip_reversed_bytes(my_ip);
    raw[8..12].copy_from_slice(&ip_bytes);
    raw[24..28].copy_from_slice(&ip_rev);
    raw[28] = 0x00;
    raw
}

pub fn get_video_init_payload_data(my_ip: &str) -> Vec<u8> {
    let mut raw = get_video_init_payload_ctrl(my_ip);
    raw[28] = 0x01;
    raw
}

pub fn get_aux_header(size: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(0xC9);
    buf.write_u32::<LittleEndian>(size).unwrap();
    buf
}

pub fn get_zero_buffer(size: u32) -> Vec<u8> {
    let mut raw = get_aux_header(size);
    raw.extend(vec![0; size as usize]);
    raw
}

pub fn get_eprd_meta_header(my_ip: &str, meta_size: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"EPRD0600");
    buf.extend_from_slice(&get_hex_ip_bytes(my_ip));
    buf.write_u32::<LittleEndian>(0).unwrap();
    buf.write_u32::<LittleEndian>(meta_size).unwrap();
    buf
}

pub fn get_eprd_jpeg_header(my_ip: &str, jpeg_size: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"EPRD0600");
    buf.extend_from_slice(&get_hex_ip_bytes(my_ip));
    buf.write_u32::<BigEndian>(0).unwrap();
    buf.write_u32::<BigEndian>(jpeg_size).unwrap();
    buf
}

pub fn get_display_config_meta() -> Vec<u8> {
    hex::decode("cc0000000400030020200001ff00ff00ff0010080000000006400384000000600400024000000000000000000000").unwrap()
}

pub fn get_frame_header(frame_type: u32, x: u16, y: u16, w: u16, h: u16) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.write_u32::<BigEndian>(frame_type).unwrap();
    buf.write_u16::<BigEndian>(x).unwrap();
    buf.write_u16::<BigEndian>(y).unwrap();
    buf.write_u16::<BigEndian>(w).unwrap();
    buf.write_u16::<BigEndian>(h).unwrap();
    buf.write_u32::<BigEndian>(0x00000007).unwrap();
    
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u32;
    buf.write_u32::<BigEndian>(ts).unwrap();
    buf
}

pub fn get_subsequent_frame_header(x: u16, y: u16, w: u16, h: u16) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.write_u16::<BigEndian>(x).unwrap();
    buf.write_u16::<BigEndian>(y).unwrap();
    buf.write_u16::<BigEndian>(w).unwrap();
    buf.write_u16::<BigEndian>(h).unwrap();
    buf.write_u32::<BigEndian>(0x00000007).unwrap();
    
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u32;
    buf.write_u32::<BigEndian>(ts).unwrap();
    buf
}
