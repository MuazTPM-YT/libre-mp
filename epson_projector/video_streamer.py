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
            self.monitor = self.sct.monitors[1]  
        else:
            print("[-] Missing dependencies for video streaming. Please run: pip install mss Pillow numpy")
    
    def start_streaming(self):
        if not DEPENDENCIES_LOADED:
            print("[-] Cannot start streaming without mss, Pillow, and numpy.")
            return

        print(f"[*] Starting video stream using mss at ~{self.target_fps} fps...")
        try:
            while True:
                start_time = time.time()
                
                sct_img = self.sct.grab(self.monitor)
                # mss provides raw BGRA bytes
                curr_frame = np.array(sct_img)
                
                x_offset, y_offset, w, h = 0, 0, self.monitor["width"], self.monitor["height"]
                delta_img = None
                
                if self.prev_frame is not None:
                    # Find differing pixels across any color channel
                    diff_mask = np.any(curr_frame != self.prev_frame, axis=2)
                    rows, cols = np.where(diff_mask)
                    
                    if len(rows) == 0:
                        # No change
                        time.sleep(self.frame_duration)
                        continue
                        
                    y_min, y_max = np.min(rows), np.max(rows)
                    x_min, x_max = np.min(cols), np.max(cols)
                    
                    y_offset, x_offset = int(y_min), int(x_min)
                    h, w = int(y_max - y_min + 1), int(x_max - x_min + 1)
                    
                    region_bgra = curr_frame[y_min:y_max+1, x_min:x_max+1]
                    # Convert BGRA -> RGBA for Pillow
                    region_rgba = region_bgra[:, :, [2, 1, 0, 3]]
                    delta_img = Image.fromarray(region_rgba, "RGBA").convert("RGB")
                    
                else:
                    # Full frame
                    curr_rgba = curr_frame[:, :, [2, 1, 0, 3]]
                    delta_img = Image.fromarray(curr_rgba, "RGBA").convert("RGB")
                
                self.prev_frame = curr_frame
                
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
