#include "wifi.hpp"
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <sstream>
#include <algorithm>
#include <array>
#include <thread>
#include <chrono>
#include <set>

// Run a command and capture stdout
static std::string exec_cmd(const std::string& cmd) {
    std::string result;
    FILE* pipe = popen(cmd.c_str(), "r");
    if (!pipe) return result;
    std::array<char, 256> buf;
    while (fgets(buf.data(), buf.size(), pipe) != nullptr) {
        result += buf.data();
    }
    pclose(pipe);
    return result;
}

// Run a command and return exit code
static int run_cmd(const std::string& cmd) {
    return system(cmd.c_str());
}

// Replace all occurrences of a substring
static std::string str_replace(std::string s, const std::string& from, const std::string& to) {
    size_t pos = 0;
    while ((pos = s.find(from, pos)) != std::string::npos) {
        s.replace(pos, from.length(), to);
        pos += to.length();
    }
    return s;
}

namespace wifi {

std::vector<WifiNetwork> scan_wifi_networks() {
    printf("Scanning for Wi-Fi networks... Please wait.\n");
    run_cmd("nmcli dev wifi rescan 2>/dev/null");
    std::this_thread::sleep_for(std::chrono::seconds(2));

    std::string output = exec_cmd("nmcli -t -f SSID,BSSID,SECURITY,SIGNAL dev wifi 2>/dev/null");

    std::vector<WifiNetwork> networks;
    std::set<std::string> seen;
    std::istringstream stream(output);
    std::string line;

    while (std::getline(stream, line)) {
        if (line.empty()) continue;

        // nmcli escapes colons as \: — handle them
        std::string processed = str_replace(line, "\\:", "%%COLON%%");

        // Split by ':'
        std::vector<std::string> parts;
        std::istringstream ss(processed);
        std::string part;
        while (std::getline(ss, part, ':')) {
            parts.push_back(str_replace(part, "%%COLON%%", ":"));
        }

        if (parts.size() >= 4) {
            std::string ssid = parts[0];
            std::string bssid = parts[1];
            std::string security = parts[2];
            int signal = 0;
            try { signal = std::stoi(parts[3]); } catch (...) {}

            if (!ssid.empty() && ssid != "--" && seen.find(ssid) == seen.end()) {
                seen.insert(ssid);
                networks.push_back({ssid, bssid, security, signal});
            }
        }
    }

    // Sort by signal descending
    std::sort(networks.begin(), networks.end(),
              [](const WifiNetwork& a, const WifiNetwork& b) { return a.signal > b.signal; });

    return networks;
}

bool connect_to_wifi(const std::string& ssid, const std::string& bssid,
                     const std::string& password) {
    printf("\nAttempting to connect to '%s' (BSSID: %s)...\n", ssid.c_str(), bssid.c_str());

    std::string cmd = "nmcli dev wifi connect '" + bssid + "'";
    if (!password.empty()) {
        cmd += " password '" + password + "'";
    }
    cmd += " 2>&1";

    std::string output = exec_cmd(cmd);
    if (output.find("successfully") != std::string::npos ||
        output.find("activated") != std::string::npos) {
        printf("[+] Successfully connected to '%s'!\n", ssid.c_str());
        return true;
    } else {
        printf("[-] Failed to connect: %s\n", output.c_str());
        return false;
    }
}

std::pair<std::string, std::string> get_current_wifi() {
    std::string output = exec_cmd("nmcli -t -f UUID,TYPE,NAME connection show --active 2>/dev/null");
    std::istringstream stream(output);
    std::string line;
    while (std::getline(stream, line)) {
        std::vector<std::string> parts;
        std::istringstream ss(line);
        std::string part;
        while (std::getline(ss, part, ':')) {
            parts.push_back(part);
        }
        if (parts.size() >= 3 && parts[1] == "802-11-wireless") {
            return {parts[0], parts[2]};
        }
    }
    return {"", ""};
}

void revert_wifi(const std::string& original_uuid, const std::string& projector_ssid) {
    if (!projector_ssid.empty()) {
        printf("\n[*] Forgetting projector network '%s'...\n", projector_ssid.c_str());
        std::string cmd = "nmcli connection delete '" + projector_ssid + "' 2>/dev/null";
        run_cmd(cmd.c_str());
    }

    if (!original_uuid.empty()) {
        printf("[*] Reconnecting to original Wi-Fi network...\n");
        std::string cmd = "nmcli connection up uuid " + original_uuid + " 2>/dev/null";
        run_cmd(cmd.c_str());
        printf("[+] Network restored.\n");
    }
}

WifiSetupResult interactive_wifi_setup() {
    WifiSetupResult result;

    auto [orig_uuid, orig_name] = get_current_wifi();
    result.original_uuid = orig_uuid;

    if (!orig_name.empty()) {
        printf("[*] Currently connected to: %s\n", orig_name.c_str());
    }

    auto networks = scan_wifi_networks();
    if (networks.empty()) {
        printf("No Wi-Fi networks found or Wi-Fi is disabled.\n");
        return result;
    }

    printf("\nAvailable Wi-Fi Networks:\n");
    for (size_t i = 0; i < networks.size(); ++i) {
        std::string sec = (networks[i].security.empty() || networks[i].security == "--")
                              ? "Open" : networks[i].security;
        printf("[%zu] %s (Signal: %d%%, Security: %s)\n",
               i + 1, networks[i].ssid.c_str(), networks[i].signal, sec.c_str());
    }

    printf("\nSelect a network to connect to (or 0 to skip): ");
    fflush(stdout);
    std::string choice;
    std::getline(std::cin, choice);

    try {
        int idx = std::stoi(choice) - 1;
        if (idx == -1) {
            printf("Skipping Wi-Fi setup.\n");
            result.connected = true;
            return result;
        }
        if (idx >= 0 && idx < static_cast<int>(networks.size())) {
            auto& net = networks[idx];
            std::string password;
            if (!net.security.empty() && net.security != "--") {
                printf("Enter password for '%s': ", net.ssid.c_str());
                fflush(stdout);
                std::getline(std::cin, password);
            }

            result.connected = connect_to_wifi(net.ssid, net.bssid, password);
            if (result.connected) {
                result.projector_ssid_to_delete = net.ssid;
            }
        } else {
            printf("Invalid choice. Skipping Wi-Fi setup.\n");
        }
    } catch (...) {
        printf("Invalid input. Skipping Wi-Fi setup.\n");
    }

    return result;
}

} // namespace wifi
