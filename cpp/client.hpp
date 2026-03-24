#pragma once
#include <string>
#include <cstdint>
#include <vector>

class EpsonEasyMPClient {
public:
    EpsonEasyMPClient(const std::string& projector_ip = "",
                      const std::string& my_ip = "");
    ~EpsonEasyMPClient();

    bool connect_and_negotiate();
    bool send_video_frame(int x_offset, int y_offset, int width, int height,
                          const uint8_t* jpeg_data, size_t jpeg_len);
    void send_keepalive();
    void disconnect();

    // Public state for VideoStreamer
    bool first_frame = true;

private:
    std::string projector_ip_;
    std::string my_ip_;

    int s_auth_      = -1;  // Port 3620
    int s_video_     = -1;  // Port 3621 - video data
    int s_video_aux_ = -1;  // Port 3621 - aux/warmup

    bool warmup_sent_ = false;

    void authenticate_session();
    void complete_auth_handshake();
    std::vector<uint8_t> build_0x0108_response();
    void open_video_channels();
    void wait_for_streaming_signal();
    void send_aux_bundle(uint32_t size);
    void send_warmup_buffers();

    // Socket helpers
    int create_tcp_socket();
    void connect_socket(int fd, const std::string& ip, uint16_t port, int timeout_sec);
    void set_recv_timeout(int fd, int sec);
    void send_all(int fd, const uint8_t* data, size_t len);
    void send_chunked(int fd, const uint8_t* data, size_t len, size_t chunk_size = 1460);
    ssize_t recv_data(int fd, uint8_t* buf, size_t len);
    void close_socket(int& fd);
};
