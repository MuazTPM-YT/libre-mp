#include "payloads.hpp"
#include <arpa/inet.h>
#include <cstring>
#include <ctime>
#include <stdexcept>

namespace payloads {

// ========================================================================
//  Helpers
// ========================================================================

std::vector<uint8_t> ip_to_bytes(const std::string& ip) {
    struct in_addr addr{};
    inet_pton(AF_INET, ip.c_str(), &addr);
    auto* p = reinterpret_cast<uint8_t*>(&addr.s_addr);
    return {p[0], p[1], p[2], p[3]};
}

std::vector<uint8_t> ip_to_bytes_reversed(const std::string& ip) {
    auto b = ip_to_bytes(ip);
    return {b[3], b[2], b[1], b[0]};
}

static uint8_t hex_char(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return 10 + c - 'a';
    if (c >= 'A' && c <= 'F') return 10 + c - 'A';
    throw std::runtime_error("Invalid hex char");
}

std::vector<uint8_t> hex_to_bytes(const std::string& hex) {
    std::vector<uint8_t> out;
    out.reserve(hex.size() / 2);
    for (size_t i = 0; i + 1 < hex.size(); i += 2) {
        out.push_back((hex_char(hex[i]) << 4) | hex_char(hex[i + 1]));
    }
    return out;
}

// Helper: IP string to hex string (network order)
static std::string ip_to_hex(const std::string& ip) {
    auto bytes = ip_to_bytes(ip);
    char buf[9];
    snprintf(buf, sizeof(buf), "%02x%02x%02x%02x", bytes[0], bytes[1], bytes[2], bytes[3]);
    return std::string(buf);
}

// Helper: IP string to reversed hex string
static std::string ip_to_hex_reversed(const std::string& ip) {
    auto bytes = ip_to_bytes_reversed(ip);
    char buf[9];
    snprintf(buf, sizeof(buf), "%02x%02x%02x%02x", bytes[0], bytes[1], bytes[2], bytes[3]);
    return std::string(buf);
}

// Helper: write uint32 LE into buffer
static void write_u32_le(std::vector<uint8_t>& buf, size_t offset, uint32_t val) {
    buf[offset + 0] = (val >>  0) & 0xFF;
    buf[offset + 1] = (val >>  8) & 0xFF;
    buf[offset + 2] = (val >> 16) & 0xFF;
    buf[offset + 3] = (val >> 24) & 0xFF;
}

// Helper: write uint32 BE into buffer
static void write_u32_be(std::vector<uint8_t>& buf, size_t offset, uint32_t val) {
    buf[offset + 0] = (val >> 24) & 0xFF;
    buf[offset + 1] = (val >> 16) & 0xFF;
    buf[offset + 2] = (val >>  8) & 0xFF;
    buf[offset + 3] = (val >>  0) & 0xFF;
}

// Helper: write uint16 BE into buffer
static void write_u16_be(std::vector<uint8_t>& buf, size_t offset, uint16_t val) {
    buf[offset + 0] = (val >> 8) & 0xFF;
    buf[offset + 1] = (val >> 0) & 0xFF;
}

// ========================================================================
//  Port 3620 – Authentication
// ========================================================================

std::vector<uint8_t> get_registration_payload(const std::string& my_ip) {
    std::string hex_ip = ip_to_hex(my_ip);
    std::string p =
        "45454d5030313030" + hex_ip + "0200000030000000007f0000b0f8ef5314000000"
        "0000000000000000000000000000000000000000000000000000000000000000";
    return hex_to_bytes(p);
}

std::vector<uint8_t> get_auth_payload_full(const std::string& my_ip,
                                            const std::string& proj_ip,
                                            const std::string& my_mac) {
    std::string hex_ip = ip_to_hex(my_ip);
    std::string hex_proj = ip_to_hex(proj_ip);
    std::string p =
        "45454d5030313030" + hex_ip + "01010000f40000000101000000380f000000000000"
        "ffffff0000000000020f0b0004000320200001ff00ff00ff00000810000000010e0000"
        + my_mac + "00000000000000000000000000000000"
        + hex_proj + "a600000005000000380000000200000004000000"
        + hex_ip + "0c00000004000000000000000100000004000000"
        "500043000b00000004000000000000001c00000000000000040000003600000001000000030000002a000000"
        + my_mac + hex_proj + "52455345415243484c4142000000000000000000000000000000000000000000"
        "0f00000004000000320000000d000000040000000200000026000000080000000010000000100000";
    return hex_to_bytes(p);
}

// ========================================================================
//  Port 3621 – Video Channel (EPRD Protocol)
// ========================================================================

std::vector<uint8_t> get_video_init_payload_ctrl(const std::string& my_ip) {
    std::string hex_ip = ip_to_hex(my_ip);
    std::string hex_ip_rev = ip_to_hex_reversed(my_ip);
    std::string p = "4550524430363030" + hex_ip + "0000000010000000d0000000"
                    + hex_ip_rev + "0000000000000000";
    return hex_to_bytes(p);
}

std::vector<uint8_t> get_video_init_payload_data(const std::string& my_ip) {
    std::string hex_ip = ip_to_hex(my_ip);
    std::string hex_ip_rev = ip_to_hex_reversed(my_ip);
    std::string p = "4550524430363030" + hex_ip + "0000000010000000d0000000"
                    + hex_ip_rev + "0100000000000000";
    return hex_to_bytes(p);
}

// ========================================================================
//  Auxiliary packets
// ========================================================================

std::vector<uint8_t> get_aux_header(uint32_t size) {
    std::vector<uint8_t> hdr(5);
    hdr[0] = 0xC9;
    write_u32_le(hdr, 1, size);
    return hdr;
}

std::vector<uint8_t> get_zero_buffer(uint32_t size) {
    auto hdr = get_aux_header(size);
    hdr.resize(5 + size, 0x00);
    return hdr;
}

// ========================================================================
//  EPRD headers
// ========================================================================

std::vector<uint8_t> get_eprd_meta_header(const std::string& my_ip, uint32_t meta_size) {
    // 20-byte EPRD0600 header, size field Little-Endian
    std::vector<uint8_t> hdr(20, 0);
    const char* magic = "EPRD0600";
    std::memcpy(hdr.data(), magic, 8);
    auto ip = ip_to_bytes(my_ip);
    std::memcpy(hdr.data() + 8, ip.data(), 4);
    // bytes 12-15: 0 (LE)
    write_u32_le(hdr, 12, 0);
    // bytes 16-19: meta_size (LE)
    write_u32_le(hdr, 16, meta_size);
    return hdr;
}

std::vector<uint8_t> get_eprd_jpeg_header(const std::string& my_ip, uint32_t jpeg_size) {
    // 20-byte EPRD0600 header, size field Big-Endian
    std::vector<uint8_t> hdr(20, 0);
    const char* magic = "EPRD0600";
    std::memcpy(hdr.data(), magic, 8);
    auto ip = ip_to_bytes(my_ip);
    std::memcpy(hdr.data() + 8, ip.data(), 4);
    // bytes 12-15: 0 (BE)
    write_u32_be(hdr, 12, 0);
    // bytes 16-19: jpeg_size (BE)
    write_u32_be(hdr, 16, jpeg_size);
    return hdr;
}

// ========================================================================
//  Display config + frame headers
// ========================================================================

std::vector<uint8_t> get_display_config_meta(int disp_w, int disp_h,
                                              int /*stream_w*/, int /*stream_h*/) {
    std::vector<uint8_t> meta(46, 0);
    meta[0] = 0xCC;
    meta[4] = 0x04; meta[5] = 0x00;
    meta[6] = 0x03; meta[7] = 0x00;
    meta[8] = 0x20; meta[9] = 0x20;
    meta[10] = 0x00; meta[11] = 0x01;
    meta[12] = 0xFF; meta[13] = 0x00;
    meta[14] = 0xFF; meta[15] = 0x00;
    meta[16] = 0xFF; meta[17] = 0x00;
    meta[18] = 0x10; meta[19] = 0x08;
    // Bytes 24-27: display resolution (Big-Endian)
    write_u16_be(meta, 24, static_cast<uint16_t>(disp_w));
    write_u16_be(meta, 26, static_cast<uint16_t>(disp_h));
    // Bytes 30-31: DPI hint
    meta[30] = 0x00; meta[31] = 0x60;
    // Bytes 32-35: hardcoded 1024x576 base plane scale
    write_u16_be(meta, 32, 0x0400);
    write_u16_be(meta, 34, 0x0240);
    return meta;
}

std::vector<uint8_t> get_frame_header(int frame_type, int x, int y, int w, int h) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    uint32_t ms = static_cast<uint32_t>(
        (static_cast<uint64_t>(ts.tv_sec) * 1000 + ts.tv_nsec / 1000000) & 0xFFFFFFFF
    );

    std::vector<uint8_t> hdr(20, 0);
    // 4-byte type (BE)
    write_u32_be(hdr, 0, static_cast<uint32_t>(frame_type));
    // 8-byte region: x, y, w, h (BE uint16 each)
    write_u16_be(hdr, 4, static_cast<uint16_t>(x));
    write_u16_be(hdr, 6, static_cast<uint16_t>(y));
    write_u16_be(hdr, 8, static_cast<uint16_t>(w));
    write_u16_be(hdr, 10, static_cast<uint16_t>(h));
    // Flags (0x00000002) + rolling timestamp
    write_u32_be(hdr, 12, 0x00000002);
    write_u32_be(hdr, 16, ms);
    return hdr;
}

} // namespace payloads
