#pragma once
#include <string>
#include <cstdint>

namespace config {

// Epson projectors default to 192.168.88.1
constexpr const char* PROJECTOR_IP = "192.168.88.1";

constexpr uint16_t PORT_CONTROL = 3620;
constexpr uint16_t PORT_VIDEO   = 3621;
constexpr uint16_t PORT_WAKE    = 3629;

// Projector's native display resolution
constexpr int PROJECTOR_DISPLAY_WIDTH  = 1600;
constexpr int PROJECTOR_DISPLAY_HEIGHT = 900;

// Stream resolution for JPEG frames
constexpr int STREAM_WIDTH  = 624;
constexpr int STREAM_HEIGHT = 416;

// JPEG quality (pcap shows ~50)
constexpr int JPEG_QUALITY = 50;

// Get local IP on the projector network
std::string get_local_ip();

} // namespace config
