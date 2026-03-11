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
    def __init__(self, client, fps=15):
        self.client = client
        self.target_fps = fps
        self.frame_duration = 1.0 / fps
        self.prev_frame = None
        
        if DEPENDENCIES_LOADED:
            self.sct = mss.mss()
            self.monitor = None
            
            # Try to find a working monitor (XGetImage() often fails on specific Wayland/X11 edge cases)
            print("[*] Probing monitors for screen capture compatibility...")
            # Start with monitor 1 (primary), then fallback through others
            for idx, m in enumerate(self.sct.monitors[1:], 1):
                try:
                    self.sct.grab(m)
                    self.monitor = m
                    print(f"[+] Successfully bound to Monitor {idx}: {m}")
                    break
                except Exception as e:
                    print(f"[-] Monitor {idx} grab failed: {e}")
                    
            if self.monitor is None:
                print("[*] Falling back to unified 'All Monitors' capture (Monitor 0)...")
                try:
                    self.monitor = self.sct.monitors[0]
                    self.sct.grab(self.monitor)
                    print("[+] Unified capture successful.")
                except Exception as e:
                    print(f"[-] Fatal: Could not capture any screen. Are you on strictly Wayland without XWayland? Error: {e}")
                    # Keep it as index 1 but we expect it to fail in the loop
                    self.monitor = self.sct.monitors[1] if len(self.sct.monitors) > 1 else self.sct.monitors[0]
        else:
            print("[-] Missing dependencies for video streaming. Please run: pip install mss Pillow numpy")
    
    def start_streaming(self):
        print(f"[*] Starting video stream using mss at ~{self.target_fps} fps...")
        
        # Test card state
        use_test_card = not DEPENDENCIES_LOADED or self.monitor is None
        if use_test_card:
            print("[!] WARNING: Valid screen capture monitor not found (Wayland mode?).")
            print("[!] Falling back to synthetic 'Test Card' video stream to verify connection.")
            width, height = 800, 600
        else:
            width, height = self.monitor["width"], self.monitor["height"]
            
        frame_idx = 0
            
        try:
            while True:
                start_time = time.time()
                
                curr_frame = None
                delta_img = None
                x_offset, y_offset, w, h = 0, 0, width, height
                
                if not use_test_card:
                    try:
                        sct_img = self.sct.grab(self.monitor)
                        curr_frame = np.array(sct_img)
                        
                        if self.prev_frame is not None:
                            diff_mask = np.any(curr_frame != self.prev_frame, axis=2)
                            rows, cols = np.where(diff_mask)
                            
                            if len(rows) == 0:
                                time.sleep(self.frame_duration)
                                continue
                                
                            y_min, y_max = np.min(rows), np.max(rows)
                            x_min, x_max = np.min(cols), np.max(cols)
                            
                            y_offset, x_offset = int(y_min), int(x_min)
                            h, w = int(y_max - y_min + 1), int(x_max - x_min + 1)
                            
                            region_bgra = curr_frame[y_min:y_max+1, x_min:x_max+1]
                            region_rgba = region_bgra[:, :, [2, 1, 0, 3]]
                            delta_img = Image.fromarray(region_rgba, "RGBA").convert("RGB")
                            
                        else:
                            curr_rgba = curr_frame[:, :, [2, 1, 0, 3]]
                            delta_img = Image.fromarray(curr_rgba, "RGBA").convert("RGB")
                            
                        self.prev_frame = curr_frame
                        
                    except Exception as e:
                        print(f"[-] Screen capture failed during stream (Wayland?): {e}")
                        print("[!] Switching to synthetic Test Card generator...")
                        use_test_card = True
                        
                if use_test_card:
                    import math
                    from PIL import ImageDraw
                    
                    # Create a test frame: moving bouncing block on a blue background
                    img = Image.new("RGB", (width, height), color=(40, 40, 100))
                    draw = ImageDraw.Draw(img)
                    
                    # Bounce logic
                    t = frame_idx * 0.1
                    bx = int(width/2 + math.sin(t) * (width/3))
                    by = int(height/2 + math.cos(t * 1.5) * (height/3))
                    
                    # Draw a bright red rectangle
                    draw.rectangle([bx-50, by-50, bx+50, by+50], fill=(255, 50, 50))
                    
                    # Draw some text/status indicators
                    for i in range(5):
                        draw.line((0, i*100, width, i*100), fill=(200, 200, 200), width=2)
                        
                    delta_img = img
                    x_offset, y_offset, w, h = 0, 0, width, height
                    frame_idx += 1
                
                # Encode MJPEG
                buf = io.BytesIO()
                delta_img.save(buf, format="JPEG", quality=75)
                mjpeg_bytes = buf.getvalue()
                
                success = self.client.send_video_frame(x_offset, y_offset, w, h, mjpeg_bytes)
                if not success:
                    print("[-] Failed to send video frame. Aborting stream.")
                    break
                    
                elapsed = time.time() - start_time
                if elapsed < self.frame_duration:
                    time.sleep(self.frame_duration - elapsed)
                    
        except KeyboardInterrupt:
            print("\n[*] Stopping video stream...")
        except Exception as e:
            print(f"[-] Video streaming error: {e}")
