#include "client.hpp"
#include "config.hpp"
#include "payloads.hpp"

#include <sys/socket.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <arpa/inet.h>
#include <unistd.h>
#include <cstring>
#include <cstdio>
#include <cerrno>
#include <vector>
#include <chrono>
#include <thread>
#include <stdexcept>

// ========================================================================
//  Socket helpers
// ========================================================================

int EpsonEasyMPClient::create_tcp_socket() {
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) {
        perror("socket");
        throw std::runtime_error("Failed to create socket");
    }
    int flag = 1;
    setsockopt(fd, IPPROTO_TCP, TCP_NODELAY, &flag, sizeof(flag));
    return fd;
}

void EpsonEasyMPClient::connect_socket(int fd, const std::string& ip, uint16_t port, int timeout_sec) {
    set_recv_timeout(fd, timeout_sec);

    struct sockaddr_in addr{};
    addr.sin_family = AF_INET;
    addr.sin_port = htons(port);
    inet_pton(AF_INET, ip.c_str(), &addr.sin_addr);

    if (connect(fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        perror("connect");
        throw std::runtime_error("Failed to connect to " + ip + ":" + std::to_string(port));
    }
}

void EpsonEasyMPClient::set_recv_timeout(int fd, int sec) {
    struct timeval tv{};
    tv.tv_sec = sec;
    tv.tv_usec = 0;
    setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));
}

void EpsonEasyMPClient::send_all(int fd, const uint8_t* data, size_t len) {
    size_t sent = 0;
    while (sent < len) {
        ssize_t n = send(fd, data + sent, len - sent, MSG_NOSIGNAL);
        if (n < 0) {
            throw std::runtime_error(std::string("send failed: ") + strerror(errno));
        }
        sent += n;
    }
}

void EpsonEasyMPClient::send_chunked(int fd, const uint8_t* data, size_t len, size_t chunk_size) {
    size_t sent = 0;
    while (sent < len) {
        size_t to_send = std::min(chunk_size, len - sent);
        send_all(fd, data + sent, to_send);
        sent += to_send;
    }
}

ssize_t EpsonEasyMPClient::recv_data(int fd, uint8_t* buf, size_t len) {
    return recv(fd, buf, len, 0);
}

void EpsonEasyMPClient::close_socket(int& fd) {
    if (fd >= 0) {
        close(fd);
        fd = -1;
    }
}

// ========================================================================
//  Constructor / Destructor
// ========================================================================

EpsonEasyMPClient::EpsonEasyMPClient(const std::string& projector_ip,
                                       const std::string& my_ip)
    : projector_ip_(projector_ip.empty() ? config::PROJECTOR_IP : projector_ip),
      my_ip_(my_ip.empty() ? config::get_local_ip() : my_ip) {}

EpsonEasyMPClient::~EpsonEasyMPClient() {
    disconnect();
}

// ========================================================================
//  Main negotiation sequence
// ========================================================================

bool EpsonEasyMPClient::connect_and_negotiate() {
    printf("[*] Starting deterministic negotiation sequence with %s...\n", projector_ip_.c_str());
    try {
        authenticate_session();
        complete_auth_handshake();
        open_video_channels();
        wait_for_streaming_signal();
        send_warmup_buffers();

        printf("\n[+] BINGO! Connection Fully Established and Ready for Video Stream!\n");
        return true;
    } catch (const std::exception& e) {
        printf("[-] Negotiation failed: %s\n", e.what());
        disconnect();
        return false;
    }
}

// ========================================================================
//  Authentication (Port 3620)
// ========================================================================

void EpsonEasyMPClient::authenticate_session() {
    printf("[*] 1. Opening Authentication Channel (Port 3620)...\n");

    // --- REGISTRATION PHASE ---
    s_auth_ = create_tcp_socket();
    connect_socket(s_auth_, projector_ip_, config::PORT_CONTROL, 5);

    printf("[*]    Sending 68-byte Registration Packet...\n");
    auto reg = payloads::get_registration_payload(my_ip_);
    send_all(s_auth_, reg.data(), reg.size());

    uint8_t buf[1024];
    ssize_t n = recv_data(s_auth_, buf, sizeof(buf));
    if (n > 0) {
        printf("[+]    Registration Resp 1: %zd bytes\n", n);
        // Try second response with short timeout
        set_recv_timeout(s_auth_, 1);
        ssize_t n2 = recv_data(s_auth_, buf, sizeof(buf));
        if (n2 > 0) {
            printf("[+]    Registration Resp 2: %zd bytes\n", n2);
        }
    } else {
        printf("[-]    Registration failed\n");
        throw std::runtime_error("Registration failed");
    }

    // Close registration, open new auth connection (matches Windows behavior)
    printf("[*]    Closing Registration channel, opening Auth channel...\n");
    close_socket(s_auth_);
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    s_auth_ = create_tcp_socket();
    connect_socket(s_auth_, projector_ip_, config::PORT_CONTROL, 5);

    // --- AUTHENTICATION SEQUENCE ---
    printf("[*]    Sending 264-Byte Full Auth Request...\n");
    auto auth = payloads::get_auth_payload_full(my_ip_, projector_ip_);
    printf("[*]    Payload length: %zu bytes\n", auth.size());
    send_all(s_auth_, auth.data(), auth.size());

    printf("[*]    Waiting for authentication response...\n");
    set_recv_timeout(s_auth_, 5);
    n = recv_data(s_auth_, buf, sizeof(buf));
    if (n > 0) {
        printf("[+]    Auth Resp: %zd bytes\n", n);
        if (n >= 51) {
            printf("[+]    Auth Status Byte: 0x%02x\n", buf[50]);
        }
        if (n == 296) {
            printf("[+]    Perfect 296-byte Auth Response received! We are IN.\n");
        }
    } else {
        printf("[-]    Auth response timed out or failed\n");
    }

    printf("[+]    Authentication phase complete.\n");
}

// ========================================================================
//  Post-auth handshake
// ========================================================================

void EpsonEasyMPClient::complete_auth_handshake() {
    printf("[*]    Completing post-auth handshake on Port 3620...\n");
    set_recv_timeout(s_auth_, 3);

    bool ready_received = false;
    bool already_responded = false;

    uint8_t buf[4096];
    for (int attempt = 0; attempt < 10; ++attempt) {
        ssize_t n = recv_data(s_auth_, buf, sizeof(buf));
        if (n <= 0) break;

        size_t offset = 0;
        while (offset + 20 <= static_cast<size_t>(n)) {
            if (memcmp(buf + offset, "EEMP0100", 8) != 0) break;

            uint32_t cmd;
            memcpy(&cmd, buf + offset + 12, 4);  // LE
            uint32_t payload_len;
            memcpy(&payload_len, buf + offset + 16, 4);  // LE
            size_t msg_len = 20 + payload_len;

            printf("[+]    Post-auth recv: cmd=0x%04x, %zu bytes\n", cmd, msg_len);

            if (cmd == 0x010E && !already_responded) {
                auto response = build_0x0108_response();
                send_all(s_auth_, response.data(), response.size());
                already_responded = true;
                printf("[+]    Sent 0x0108 response: %zu bytes\n", response.size());
            } else if (cmd == 0x010E && already_responded) {
                printf("[*]    Ignoring subsequent 0x010E (no response needed)\n");
            } else if (cmd == 0x0110) {
                printf("[+]    Received 0x0110 'Ready to Stream' signal!\n");
                ready_received = true;
                break;
            }

            offset += msg_len;
        }

        if (ready_received) break;
    }

    set_recv_timeout(s_auth_, 5);

    if (ready_received) {
        printf("[+]    Post-auth handshake complete. Projector is ready!\n");
    } else {
        printf("[*]    Post-auth handshake complete (no explicit ready signal, continuing).\n");
    }
}

std::vector<uint8_t> EpsonEasyMPClient::build_0x0108_response() {
    // Exact 348-byte payload from PCAP Frame 124
    std::string pcap_hex =
        "45454d5030313030c0a858020801000048010000"
        "0001000000000000000000000000000000000000"
        "00000000000000000000000000000000000000000000000000000000"
        "1401000005000000380000000200000004000000"
        "c0a858020c00000004000000010000000100000004000000"
        "500043000b00000004000000000000001c00000000000000"
        "07000000440000000100000005000000380000000200000004000000"
        "c0a858020c00000004000000010000000100000004000000"
        "500043000b00000004000000000000001c00000000000000"
        "08000000800000000400000005000000380000000200000004000000"
        "c0a858020c00000004000000010000000100000004000000"
        "500043000b00000004000000010100001c00000000000000"
        "000000000c000000020000000400000002000000"
        "000000000c000000020000000400000003000000"
        "000000000c000000020000000400000004000000";

    auto raw = payloads::hex_to_bytes(pcap_hex);

    // Replace all occurrences of pcap client IP (192.168.88.2) with our actual IP
    auto my_ip_bytes = payloads::ip_to_bytes(my_ip_);
    auto pcap_ip_bytes = payloads::ip_to_bytes("192.168.88.2");

    for (size_t i = 0; i + 3 < raw.size(); ++i) {
        if (raw[i]   == pcap_ip_bytes[0] &&
            raw[i+1] == pcap_ip_bytes[1] &&
            raw[i+2] == pcap_ip_bytes[2] &&
            raw[i+3] == pcap_ip_bytes[3]) {
            raw[i]   = my_ip_bytes[0];
            raw[i+1] = my_ip_bytes[1];
            raw[i+2] = my_ip_bytes[2];
            raw[i+3] = my_ip_bytes[3];
        }
    }

    return raw;
}

// ========================================================================
//  Video channels (Port 3621)
// ========================================================================

void EpsonEasyMPClient::open_video_channels() {
    printf("[*] 2. Opening Video Channels (Port 3621)...\n");
    std::this_thread::sleep_for(std::chrono::milliseconds(300));

    // Connection 1: VIDEO DATA (byte 28 = 0x00)
    s_video_ = create_tcp_socket();
    connect_socket(s_video_, projector_ip_, config::PORT_VIDEO, 5);
    // Clear recv timeout for blocking mode
    set_recv_timeout(s_video_, 0);

    auto ctrl_init = payloads::get_video_init_payload_ctrl(my_ip_);
    send_all(s_video_, ctrl_init.data(), ctrl_init.size());
    printf("[+]    Video channel OPEN (EPRD init byte28=0x00)\n");

    // Connection 2: AUX / WARMUP / KEEPALIVE (byte 28 = 0x01)
    s_video_aux_ = create_tcp_socket();
    connect_socket(s_video_aux_, projector_ip_, config::PORT_VIDEO, 5);
    set_recv_timeout(s_video_aux_, 0);

    auto data_init = payloads::get_video_init_payload_data(my_ip_);
    send_all(s_video_aux_, data_init.data(), data_init.size());
    printf("[+]    Aux channel OPEN (EPRD init byte28=0x01)\n");

    first_frame = true;
    warmup_sent_ = false;
}

// ========================================================================
//  Wait for 0x0016
// ========================================================================

void EpsonEasyMPClient::wait_for_streaming_signal() {
    printf("[*] 3. Waiting for projector 0x0016 streaming signal...\n");
    set_recv_timeout(s_auth_, 10);

    uint8_t buf[4096];
    ssize_t n = recv_data(s_auth_, buf, sizeof(buf));
    if (n >= 20 && memcmp(buf, "EEMP0100", 8) == 0) {
        uint32_t cmd;
        memcpy(&cmd, buf + 12, 4);
        printf("[+]    Received cmd=0x%04x (%zd bytes) from projector\n", cmd, n);
        if (cmd == 0x0016) {
            printf("[+]    Projector confirmed streaming ready (0x0016)!\n");
        } else {
            printf("[*]    Expected 0x0016, got 0x%04x. Continuing anyway...\n", cmd);
        }
    } else if (n > 0) {
        printf("[*]    Received %zd bytes (not EEMP). Continuing...\n", n);
    } else {
        printf("[*]    No 0x0016 received within timeout. Continuing anyway...\n");
    }

    set_recv_timeout(s_auth_, 5);
}

// ========================================================================
//  Warmup + aux
// ========================================================================

void EpsonEasyMPClient::send_aux_bundle(uint32_t size) {
    if (s_video_aux_ < 0) return;

    auto hdr = payloads::get_aux_header(size);
    send_all(s_video_aux_, hdr.data(), hdr.size());

    // Send zero payload
    std::vector<uint8_t> zeros(size, 0);
    send_all(s_video_aux_, zeros.data(), zeros.size());
}

void EpsonEasyMPClient::send_warmup_buffers() {
    if (warmup_sent_) return;

    printf("[*] 4. Sending warmup buffers on aux channel...\n");
    const uint32_t warmup_sizes[] = {7276, 2646, 1764};

    for (int i = 0; i < 3; ++i) {
        send_aux_bundle(warmup_sizes[i]);
        printf("[+]    Warmup buffer %d: %u zeros on aux channel\n", i + 1, warmup_sizes[i]);
        std::this_thread::sleep_for(std::chrono::milliseconds(50));
    }

    warmup_sent_ = true;
    printf("[+]    Warmup complete.\n");
}

// ========================================================================
//  Video frame sending
// ========================================================================

bool EpsonEasyMPClient::send_video_frame(int x_offset, int y_offset, int width, int height,
                                          const uint8_t* jpeg_data, size_t jpeg_len) {
    if (s_video_ < 0) {
        printf("[-] Cannot send video frame. Video socket not initialized!\n");
        return false;
    }

    try {
        if (first_frame) {
            auto meta = payloads::get_display_config_meta(
                config::PROJECTOR_DISPLAY_WIDTH, config::PROJECTOR_DISPLAY_HEIGHT);
            auto meta_hdr = payloads::get_eprd_meta_header(my_ip_, meta.size());

            auto frame_hdr = payloads::get_frame_header(4, x_offset, y_offset, width, height);
            size_t jpeg_payload_size = frame_hdr.size() + jpeg_len;
            auto jpeg_hdr = payloads::get_eprd_jpeg_header(my_ip_, jpeg_payload_size);

            printf("[*]    Sending first frame: meta_block=66, jpeg_block=%zu\n", jpeg_payload_size);

            // 1. Meta Block (66 bytes total)
            std::vector<uint8_t> meta_block;
            meta_block.reserve(meta_hdr.size() + meta.size());
            meta_block.insert(meta_block.end(), meta_hdr.begin(), meta_hdr.end());
            meta_block.insert(meta_block.end(), meta.begin(), meta.end());
            send_all(s_video_, meta_block.data(), meta_block.size());

            std::this_thread::sleep_for(std::chrono::milliseconds(5));

            // 2. JPEG Block — single contiguous buffer, chunked to MSS
            std::vector<uint8_t> jpeg_block;
            jpeg_block.reserve(jpeg_hdr.size() + frame_hdr.size() + jpeg_len);
            jpeg_block.insert(jpeg_block.end(), jpeg_hdr.begin(), jpeg_hdr.end());
            jpeg_block.insert(jpeg_block.end(), frame_hdr.begin(), frame_hdr.end());
            jpeg_block.insert(jpeg_block.end(), jpeg_data, jpeg_data + jpeg_len);
            send_chunked(s_video_, jpeg_block.data(), jpeg_block.size(), 1460);

            first_frame = false;
        } else {
            auto frame_hdr = payloads::get_frame_header(3, x_offset, y_offset, width, height);
            size_t jpeg_payload_size = frame_hdr.size() + jpeg_len;
            auto jpeg_hdr = payloads::get_eprd_jpeg_header(my_ip_, jpeg_payload_size);

            std::vector<uint8_t> jpeg_block;
            jpeg_block.reserve(jpeg_hdr.size() + frame_hdr.size() + jpeg_len);
            jpeg_block.insert(jpeg_block.end(), jpeg_hdr.begin(), jpeg_hdr.end());
            jpeg_block.insert(jpeg_block.end(), frame_hdr.begin(), frame_hdr.end());
            jpeg_block.insert(jpeg_block.end(), jpeg_data, jpeg_data + jpeg_len);
            send_chunked(s_video_, jpeg_block.data(), jpeg_block.size(), 1460);
        }

        return true;
    } catch (const std::exception& e) {
        printf("[-] Stream interrupted: %s\n", e.what());
        printf("[-]    Error: %s (errno=%d)\n", strerror(errno), errno);
        if (first_frame) {
            printf("[-]    Failed on FIRST frame (TCP segment mismatch?)\n");
        }
        return false;
    }
}

void EpsonEasyMPClient::send_keepalive() {
    if (s_video_aux_ < 0) return;
    try {
        send_aux_bundle(2646);
    } catch (...) {}
}

// ========================================================================
//  Disconnect
// ========================================================================

void EpsonEasyMPClient::disconnect() {
    printf("\n[*] Disconnecting client...\n");
    close_socket(s_auth_);
    close_socket(s_video_);
    close_socket(s_video_aux_);
}
