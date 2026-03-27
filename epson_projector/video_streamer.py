import time
import io
import os
import subprocess

try:
    from PIL import Image
    PIL_LOADED = True
except ImportError:
    PIL_LOADED = False

class VideoStreamer:
    """
    Captures the screen and sends JPEG frames to the projector via the
    Epson EPRD protocol.
    
    On Wayland (Hyprland, Sway, etc.) uses 'grim' exclusively since mss
    cannot capture on Wayland compositors.
    """
    
    def __init__(self, client, fps=15):
        self.client = client
        self.target_fps = fps
        self.frame_duration = 1.0 / fps
        self.frame_idx = 0
        
        from . import config
        self.stream_width = config.STREAM_WIDTH
        self.stream_height = config.STREAM_HEIGHT
        self.jpeg_quality = config.JPEG_QUALITY
        
        # Determine capture method
        self.capture_method = None  # 'grim', 'mss', or None
        
        is_wayland = os.environ.get('XDG_SESSION_TYPE') == 'wayland' or \
                     os.environ.get('WAYLAND_DISPLAY') is not None
        
        if is_wayland:
            # On Wayland, ONLY use grim — mss produces blank frames
            if self._check_grim():
                self.capture_method = self._capture_grim
                print(f"[+] Wayland detected. Using 'grim' for screen capture.")
            else:
                print("[-] Wayland detected but 'grim' not found!")
                print("    Install it: sudo pacman -S grim")
        else:
            # On X11, try mss first, then fall back to grim
            if self._try_mss():
                self.capture_method = self._capture_mss
                print(f"[+] Using mss for screen capture (X11).")
            elif self._check_grim():
                self.capture_method = self._capture_grim
                print(f"[+] Using grim fallback for screen capture.")
        
        if self.capture_method is None:
            print("[-] No working screen capture method found!")
            if not PIL_LOADED:
                print("    Also missing Pillow. Run: pip install Pillow")

    def _check_grim(self):
        """Check if grim is installed and working."""
        try:
            result = subprocess.run(['which', 'grim'], stdout=subprocess.DEVNULL, 
                                   stderr=subprocess.DEVNULL)
            return result.returncode == 0
        except Exception:
            return False
    
    def _try_mss(self):
        """Try to initialize mss (only works on X11)."""
        try:
            import mss
            self.sct = mss.mss()
            for idx, m in enumerate(self.sct.monitors[1:], 1):
                try:
                    self.sct.grab(m)
                    self.monitor = m
                    print(f"[+] mss bound to Monitor {idx}: {m}")
                    return True
                except Exception:
                    pass
            # Try unified monitor
            try:
                self.monitor = self.sct.monitors[0]
                self.sct.grab(self.monitor)
                return True
            except Exception:
                pass
        except Exception:
            pass
        return False

    def _capture_grim(self):
        """Capture screen using grim (Wayland native), return PIL Image.
        
        Captures as PNG (lossless) then resizes to exact stream dimensions.
        JPEG encoding with correct subsampling is done later in start_streaming.
        """
        proc = subprocess.run(
            ['grim', '-t', 'png', '-'],
            capture_output=True
        )
        if proc.returncode != 0:
            return None
        img = Image.open(io.BytesIO(proc.stdout))
        return img.resize((self.stream_width, self.stream_height), Image.LANCZOS)

    def _capture_mss(self):
        """Capture screen using mss (X11), return PIL Image."""
        import numpy as np
        sct_img = self.sct.grab(self.monitor)
        raw = np.array(sct_img)
        img = Image.fromarray(raw[:, :, [2, 1, 0]], "RGB")
        return img.resize((self.stream_width, self.stream_height), Image.LANCZOS)

    def _capture_screen(self):
        """Bypass PIL completely. Return a solid blue raw RGB block."""
        return bytes([0, 0, 255] * (self.stream_width * self.stream_height))

    def _make_test_card(self):
        """Generate a synthetic test card frame."""
        import math
        from PIL import ImageDraw
        
        w, h = self.stream_width, self.stream_height
        img = Image.new("RGB", (w, h), color=(40, 40, 100))
        draw = ImageDraw.Draw(img)
        
        t = self.frame_idx * 0.1
        bx = int(w / 2 + math.sin(t) * (w / 3))
        by = int(h / 2 + math.cos(t * 1.5) * (h / 3))
        
        draw.rectangle([bx - 50, by - 50, bx + 50, by + 50], fill=(255, 50, 50))
        for i in range(5):
            draw.line((0, i * 100, w, i * 100), fill=(200, 200, 200), width=2)
        try:
            draw.text((10, 10), f"Frame: {self.frame_idx}", fill=(255, 255, 255))
        except Exception:
            pass
        return img

    def start_streaming(self):
        print(f"[*] Starting video stream at ~{self.target_fps} fps...")
        print(f"[*] Stream resolution: {self.stream_width}x{self.stream_height}, "
              f"JPEG quality: {self.jpeg_quality}")
        
        use_test_card = self.capture_method is None
        if use_test_card:
            print("[!] No screen capture available. Using test card.")
        
        try:
            while True:
                start_time = time.time()
                
                # Capture the screen frame and encode it as JPEG
                frame = self.capture_method()
                buf = io.BytesIO()
                # frame.save(buf, format="JPEG", quality=self.jpeg_quality)
                frame.save(buf, format="JPEG", quality=self.jpeg_quality, subsampling=2, optimize=False, progressive=False)
                jpeg_bytes = buf.getvalue()
                
                # Send via EPRD protocol
                success = self.client.send_video_frame(
                    0, 0,
                    self.stream_width, self.stream_height,
                    jpeg_bytes
                )
                if not success:
                    print("[-] Failed to send video frame. Aborting stream.")
                    break
                
                self.frame_idx += 1
                
                # Send AUX keepalive every ~1 second (every target_fps frames)
                # Windows PCAP shows keepalives at ~1s intervals, not per-frame
                if self.frame_idx % self.target_fps == 0:
                    self.client._send_frame_keepalive()
                
                elapsed = time.time() - start_time
                if elapsed < self.frame_duration:
                    time.sleep(self.frame_duration - elapsed)
                    
        except KeyboardInterrupt:
            print("\n[*] Stopping video stream...")
        except Exception as e:
            print(f"[-] Video streaming error: {e}")
            import traceback
            traceback.print_exc()

    # def start_streaming(self):
    #     print(f"\n[*] ISOLATION TEST: Capturing and sending exactly 1 frame...")
        
    #     # 1. Capture
    #     frame = self.capture_method()
        
    #     # 2. Sanity Check: Save locally
    #     frame.save("debug_frame.jpg")
    #     print("[+] Saved 'debug_frame.jpg' to your folder.")
    #     print("[!] OPEN 'debug_frame.jpg' NOW to confirm it is not a black box!")
        
    #     # 3. Hardcode the dumbest, safest JPEG possible
    #     buf = io.BytesIO()
    #     frame.save(buf, format="JPEG", quality=self.jpeg_quality, subsampling=2, optimize=False, progressive=False)
    #     jpeg_bytes = buf.getvalue()
        
    #     # 4. Fire the single frame
    #     self.client.send_video_frame(0, 0, self.stream_width, self.stream_height, jpeg_bytes)
        
    #     print("[*] Frame sent! Holding TCP connection open for 15 seconds...")
    #     print("[*] Watch the projector. Give the decoder time to flip the buffer.")
    #     time.sleep(15)