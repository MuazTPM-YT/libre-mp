#pragma once
#include <cstdint>
#include <string>
#include <vector>

namespace payloads {

// Helper: convert IP string to 4-byte network-order vector
std::vector<uint8_t> ip_to_bytes(const std::string& ip);

// Helper: convert IP string to 4-byte reversed vector
std::vector<uint8_t> ip_to_bytes_reversed(const std::string& ip);

// Helper: convert hex string to byte vector
std::vector<uint8_t> hex_to_bytes(const std::string& hex);

// Port 3620 - Authentication
std::vector<uint8_t> get_registration_payload(const std::string& my_ip);
std::vector<uint8_t> get_auth_payload_full(const std::string& my_ip,
                                            const std::string& proj_ip,
                                            const std::string& my_mac = "a4d73ccdaf45");

// Port 3621 - Video Channel (EPRD Protocol)
std::vector<uint8_t> get_video_init_payload_ctrl(const std::string& my_ip);
std::vector<uint8_t> get_video_init_payload_data(const std::string& my_ip);

// Auxiliary packets
std::vector<uint8_t> get_aux_header(uint32_t size);
std::vector<uint8_t> get_zero_buffer(uint32_t size);

// EPRD headers
std::vector<uint8_t> get_eprd_meta_header(const std::string& my_ip, uint32_t meta_size);
std::vector<uint8_t> get_eprd_jpeg_header(const std::string& my_ip, uint32_t jpeg_size);

// Display config + frame headers
std::vector<uint8_t> get_display_config_meta(int disp_w = 1600, int disp_h = 900,
                                              int stream_w = 624, int stream_h = 416);
std::vector<uint8_t> get_frame_header(int frame_type, int x, int y, int w, int h);

} // namespace payloads
