#pragma once
#include <vector>
#include <cstdint>

class EpsonEasyMPClient;

class VideoStreamer {
public:
    VideoStreamer(EpsonEasyMPClient& client, int fps = 15);
    void start_streaming();

private:
    EpsonEasyMPClient& client_;
    int target_fps_;
    double frame_duration_;
    int frame_idx_ = 0;
    int stream_width_;
    int stream_height_;
    int jpeg_quality_;

    enum CaptureMethod { NONE, GRIM };
    CaptureMethod capture_method_ = NONE;

    bool check_grim();

    // Capture screen via grim as raw PPM, then resize and encode JPEG.
    // Returns JPEG bytes, or empty vector on failure.
    // out_width/out_height are set to (stream_width_, stream_height_).
    std::vector<uint8_t> capture_and_encode();
};
