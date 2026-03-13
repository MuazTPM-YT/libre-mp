import time
import io
import numpy as np

try:
    import mss
    from PIL import Image
    DEPENDENCIES_LOADED = True
except ImportError:
    DEPENDENCIES_LOADED = False

class VideoStreamer:
    """
    Captures the screen and sends JPEG frames to the projector via the
    Epson EPRD protocol.
    
    Key protocol details from pcap analysis:
    - The projector expects JPEG images at a SCALED-DOWN resolution (~624x416)
    - First frame = keyframe (type 4), subsequent frames = delta (type 3)
    - Zero-buffer keepalives are sent periodically between frames
    """
    
    def __init__(self, client, fps=15):
        self.client = client
        self.target_fps = fps
        self.frame_duration = 1.0 / fps
        self.prev_frame = None
        self.use_grim = False
        self.frame_idx = 0
        
        # Import config for resolution settings
        from . import config
        self.stream_width = config.STREAM_WIDTH
        self.stream_height = config.STREAM_HEIGHT
        self.jpeg_quality = config.JPEG_QUALITY
        
        # Check if grim is installed (for Wayland fallback)
        import subprocess
        try:
            if subprocess.run(['which', 'grim'], stdout=subprocess.DEVNULL).returncode == 0:
                self.use_grim = True
                print("[+] Wayland 'grim' screenshot utility detected. Capture is supported!")
        except Exception:
            pass

        if DEPENDENCIES_LOADED:
            self.sct = mss.mss()
            self.monitor = None
            
            # Try to find a working monitor
            print("[*] Probing monitors for screen capture compatibility...")
            for idx, m in enumerate(self.sct.monitors[1:], 1):
                try:
                    self.sct.grab(m)
                    self.monitor = m
                    print(f"[+] Successfully bound to Monitor {idx}: {m}")
                    break
                except Exception:
                    pass
                    
            if self.monitor is None:
                try:
                    self.monitor = self.sct.monitors[0]
                    self.sct.grab(self.monitor)
                    print("[+] Unified capture successful.")
                except Exception:
                    print(f"[-] mss grab failed (Wayland expected), using grim fallback if available.")
                    self.monitor = None
        else:
            print("[-] Missing dependencies for video streaming. Please run: pip install mss Pillow numpy")
    
    def _capture_screen(self):
        """Capture the screen and return a PIL Image resized to stream resolution."""
        img = None
        
        if self.monitor is not None:
            sct_img = self.sct.grab(self.monitor)
            # mss returns BGRA, convert to RGB PIL Image
            raw = np.array(sct_img)
            img = Image.fromarray(raw[:, :, [2, 1, 0]], "RGB")
        elif self.use_grim:
            import subprocess
            proc = subprocess.run(
                ['grim', '-t', 'ppm', '-'],
                capture_output=True, check=True
            )
            img = Image.open(io.BytesIO(proc.stdout)).convert("RGB")
        
        if img is not None:
            # Scale to the projector's expected stream resolution
            img = img.resize((self.stream_width, self.stream_height), Image.LANCZOS)
        
        return img

    def _make_test_card(self, width, height):
        """Generate a synthetic test card frame."""
        import math
        from PIL import ImageDraw
        
        img = Image.new("RGB", (width, height), color=(40, 40, 100))
        draw = ImageDraw.Draw(img)
        
        t = self.frame_idx * 0.1
        bx = int(width / 2 + math.sin(t) * (width / 3))
        by = int(height / 2 + math.cos(t * 1.5) * (height / 3))
        
        draw.rectangle([bx - 50, by - 50, bx + 50, by + 50], fill=(255, 50, 50))
        
        for i in range(5):
            draw.line((0, i * 100, width, i * 100), fill=(200, 200, 200), width=2)
        
        # Add frame counter text
        try:
            draw.text((10, 10), f"Frame: {self.frame_idx}", fill=(255, 255, 255))
        except Exception:
            pass
            
        return img

    def start_streaming(self):
        print(f"[*] Starting video stream at ~{self.target_fps} fps...")
        print(f"[*] Stream resolution: {self.stream_width}x{self.stream_height}, JPEG quality: {self.jpeg_quality}")
        
        use_test_card = False
        if (not DEPENDENCIES_LOADED) or (self.monitor is None and not self.use_grim):
            use_test_card = True
            print("[!] WARNING: Valid screen capture method not found.")
            print("[!] Falling back to synthetic 'Test Card' video stream to verify connection.")
        
        keepalive_counter = 0
        
        try:
            while True:
                start_time = time.time()
                
                try:
                    if use_test_card:
                        img = self._make_test_card(self.stream_width, self.stream_height)
                    else:
                        img = self._capture_screen()
                        if img is None:
                            print("[!] Capture returned None, switching to test card")
                            use_test_card = True
                            continue
                except Exception as e:
                    print(f"[-] Screen capture failed: {e}")
                    print("[!] Switching to test card generator...")
                    use_test_card = True
                    continue
                
                # Encode as JPEG
                buf = io.BytesIO()
                img.save(buf, format="JPEG", quality=self.jpeg_quality)
                jpeg_bytes = buf.getvalue()
                
                # Send the frame using the real EPRD protocol
                success = self.client.send_video_frame(
                    0, 0,  # x_offset, y_offset (full frame)
                    self.stream_width, self.stream_height,
                    jpeg_bytes
                )
                if not success:
                    print("[-] Failed to send video frame. Aborting stream.")
                    break
                
                self.frame_idx += 1
                
                # Send periodic keepalive (every ~10 frames, matching pcap pattern)
                keepalive_counter += 1
                if keepalive_counter >= 10:
                    self.client.send_keepalive()
                    keepalive_counter = 0
                
                elapsed = time.time() - start_time
                if elapsed < self.frame_duration:
                    time.sleep(self.frame_duration - elapsed)
                    
        except KeyboardInterrupt:
            print("\n[*] Stopping video stream...")
        except Exception as e:
            print(f"[-] Video streaming error: {e}")
            import traceback
            traceback.print_exc()
