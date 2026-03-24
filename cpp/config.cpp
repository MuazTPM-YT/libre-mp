#include "config.hpp"
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <unistd.h>
#include <cstring>

namespace config {

std::string get_local_ip() {
    // Same trick as Python: create a UDP socket to the projector
    // to discover our routable local IP
    int fd = socket(AF_INET, SOCK_DGRAM, 0);
    if (fd < 0) return "192.168.88.2";

    struct sockaddr_in dest{};
    dest.sin_family = AF_INET;
    dest.sin_port = htons(80);
    inet_pton(AF_INET, PROJECTOR_IP, &dest.sin_addr);

    if (connect(fd, (struct sockaddr*)&dest, sizeof(dest)) < 0) {
        close(fd);
        return "192.168.88.2";
    }

    struct sockaddr_in local{};
    socklen_t len = sizeof(local);
    getsockname(fd, (struct sockaddr*)&local, &len);
    close(fd);

    char buf[INET_ADDRSTRLEN];
    inet_ntop(AF_INET, &local.sin_addr, buf, sizeof(buf));
    return std::string(buf);
}

} // namespace config
