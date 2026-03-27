#include "video_streamer.hpp"
#include "client.hpp"
#include "config.hpp"

#include <chrono>
#include <csignal>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <thread>
#include <turbojpeg.h>
#include <vector>

// Global flag for clean shutdown
static volatile sig_atomic_t g_running = 1;
static void signal_handler(int) { g_running = 0; }

VideoStreamer::VideoStreamer(EpsonEasyMPClient &client, int fps)
    : client_(client), target_fps_(fps), frame_duration_(1.0 / fps),
      stream_width_(config::STREAM_WIDTH),
      stream_height_(config::STREAM_HEIGHT),
      jpeg_quality_(config::JPEG_QUALITY) {

  if (check_grim()) {
    capture_method_ = GRIM;
    printf("[+] Wayland detected. Using 'grim' for screen capture.\n");
  } else {
    printf("[-] 'grim' not found! Install it: sudo pacman -S grim\n");
  }
}

bool VideoStreamer::check_grim() {
  return system("which grim >/dev/null 2>&1") == 0;
}

// ========================================================================
//  Screen capture + JPEG encode pipeline
//  Uses grim -> raw PPM -> libturbojpeg resize+compress
//  This is MUCH faster than Python's grim -> PNG -> PIL decode -> resize ->
//  JPEG
// ========================================================================

std::vector<uint8_t> VideoStreamer::capture_and_encode() {
  // Capture screen as raw PPM (no PNG decode overhead!)
  // grim -t ppm outputs: "P6\n<width> <height>\n255\n<raw RGB bytes>"
  FILE *pipe = popen("grim -t ppm -", "r");
  if (!pipe)
    return {};

  // Read entire PPM into memory
  std::vector<uint8_t> ppm_data;
  ppm_data.reserve(1920 * 1080 * 3 + 64); // typical screen size
  uint8_t buf[65536];
  size_t n;
  while ((n = fread(buf, 1, sizeof(buf), pipe)) > 0) {
    ppm_data.insert(ppm_data.end(), buf, buf + n);
  }
  int status = pclose(pipe);
  if (status != 0 || ppm_data.size() < 16)
    return {};

  // Parse PPM header: "P6\n<width> <height>\n<maxval>\n"
  // Find the three newlines
  size_t pos = 0;
  // Skip "P6\n"
  while (pos < ppm_data.size() && ppm_data[pos] != '\n')
    pos++;
  pos++; // skip first newline

  // Parse width and height
  int cap_width = 0, cap_height = 0;
  while (pos < ppm_data.size() && ppm_data[pos] >= '0' &&
         ppm_data[pos] <= '9') {
    cap_width = cap_width * 10 + (ppm_data[pos] - '0');
    pos++;
  }
  pos++; // skip space
  while (pos < ppm_data.size() && ppm_data[pos] >= '0' &&
         ppm_data[pos] <= '9') {
    cap_height = cap_height * 10 + (ppm_data[pos] - '0');
    pos++;
  }
  pos++; // skip newline

  // Skip maxval line
  while (pos < ppm_data.size() && ppm_data[pos] != '\n')
    pos++;
  pos++; // skip newline

  if (cap_width <= 0 || cap_height <= 0)
    return {};

  const uint8_t *src_pixels = ppm_data.data() + pos;
  size_t src_size = ppm_data.size() - pos;
  size_t expected = (size_t)cap_width * cap_height * 3;
  if (src_size < expected)
    return {};

  // Resize using nearest-neighbor (fast!) to stream dimensions
  // For better quality, we do a simple bilinear-ish downsample
  std::vector<uint8_t> resized(stream_width_ * stream_height_ * 3);

  float x_ratio = (float)cap_width / stream_width_;
  float y_ratio = (float)cap_height / stream_height_;

  for (int y = 0; y < stream_height_; ++y) {
    int src_y = (int)(y * y_ratio);
    if (src_y >= cap_height)
      src_y = cap_height - 1;
    for (int x = 0; x < stream_width_; ++x) {
      int src_x = (int)(x * x_ratio);
      if (src_x >= cap_width)
        src_x = cap_width - 1;
      size_t src_idx = ((size_t)src_y * cap_width + src_x) * 3;
      size_t dst_idx = ((size_t)y * stream_width_ + x) * 3;
      resized[dst_idx + 0] = src_pixels[src_idx + 0];
      resized[dst_idx + 1] = src_pixels[src_idx + 1];
      resized[dst_idx + 2] = src_pixels[src_idx + 2];
    }
  }

  // Encode to JPEG using libturbojpeg (hardware accelerated, 4:2:0)
  tjhandle tj = tjInitCompress();
  if (!tj)
    return {};

  unsigned char *jpeg_buf = nullptr;
  unsigned long jpeg_size = 0;

  int ret = tjCompress2(tj, resized.data(), stream_width_,
                        stream_width_ * 3, // pitch
                        stream_height_, TJPF_RGB, &jpeg_buf, &jpeg_size,
                        TJSAMP_420, jpeg_quality_, TJFLAG_FASTDCT);

  std::vector<uint8_t> result;
  if (ret == 0 && jpeg_buf) {
    result.assign(jpeg_buf, jpeg_buf + jpeg_size);
  }

  tjFree(jpeg_buf);
  tjDestroy(tj);

  return result;
}

// ========================================================================
//  Main streaming loop
// ========================================================================

void VideoStreamer::start_streaming() {
  printf("[*] Starting video stream at ~%d fps...\n", target_fps_);
  printf("[*] Stream resolution: %dx%d, JPEG quality: %d\n", stream_width_,
         stream_height_, jpeg_quality_);

  if (capture_method_ == NONE) {
    printf("[-] No screen capture method available. Cannot stream.\n");
    return;
  }

  // Install signal handler for clean Ctrl+C
  struct sigaction sa{};
  sa.sa_handler = signal_handler;
  sigaction(SIGINT, &sa, nullptr);

  while (g_running) {
    auto start = std::chrono::steady_clock::now();

    auto jpeg = capture_and_encode();
    if (jpeg.empty()) {
      printf("[-] Screen capture failed\n");
      continue;
    }

    bool ok = client_.send_video_frame(0, 0, stream_width_, stream_height_,
                                       jpeg.data(), jpeg.size());

    if (!ok) {
      printf("[-] Failed to send video frame. Aborting stream.\n");
      break;
    }

    frame_idx_++;

    auto elapsed = std::chrono::steady_clock::now() - start;
    double elapsed_sec = std::chrono::duration<double>(elapsed).count();
    if (elapsed_sec < frame_duration_) {
      std::this_thread::sleep_for(
          std::chrono::duration<double>(frame_duration_ - elapsed_sec));
    }
  }

  if (!g_running) {
    printf("\n[*] Stopping video stream...\n");
  }
}
