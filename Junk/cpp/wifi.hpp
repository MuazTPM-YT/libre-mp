#pragma once
#include <string>
#include <vector>

struct WifiNetwork {
    std::string ssid;
    std::string bssid;
    std::string security;
    int signal = 0;
};

struct WifiSetupResult {
    bool connected = false;
    std::string original_uuid;
    std::string projector_ssid_to_delete;
};

namespace wifi {

std::vector<WifiNetwork> scan_wifi_networks();
bool connect_to_wifi(const std::string& ssid, const std::string& bssid,
                     const std::string& password = "");
std::pair<std::string, std::string> get_current_wifi();  // uuid, name
void revert_wifi(const std::string& original_uuid, const std::string& projector_ssid);
WifiSetupResult interactive_wifi_setup();

} // namespace wifi
