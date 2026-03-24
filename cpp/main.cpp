#include "client.hpp"
#include "wifi.hpp"
#include "video_streamer.hpp"

#include <cstdio>
#include <cstdlib>
#include <csignal>

int main() {
    printf("--- Wi-Fi Setup ---\n");
    auto wifi_result = wifi::interactive_wifi_setup();

    if (!wifi_result.connected) {
        printf("Warning: Network setup was skipped or failed. Continuing anyway.\n");
    }

    EpsonEasyMPClient* client = nullptr;

    try {
        printf("\n--- Initializing Projector Client ---\n");
        client = new EpsonEasyMPClient();

        bool success = client->connect_and_negotiate();

        if (success) {
            VideoStreamer streamer(*client, 24);
            streamer.start_streaming();
        }

        client->disconnect();
    } catch (...) {
        printf("\n[*] Exiting...\n");
        if (client) {
            try { client->disconnect(); } catch (...) {}
        }
    }

    delete client;

    printf("\n--- Clean Up ---\n");
    wifi::revert_wifi(wifi_result.original_uuid, wifi_result.projector_ssid_to_delete);

    return 0;
}
