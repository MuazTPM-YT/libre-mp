import time
import io
import os
import struct
import subprocess

try:
    from PIL import Image
    PIL_LOADED = True
except ImportError:
    PIL_LOADED = False


TEMPLATE_FILE = os.path.join(os.path.dirname(os.path.dirname(__file__)),
                             "windows_perfect_stream.bin")


def pad_jpeg(jpeg_bytes, target_size):
    """Pad JPEG to exact target_size using JPEG COM markers before FFD9."""
    if len(jpeg_bytes) >= target_size:
        # Truncate: insert FFD9 at target_size-2
        return jpeg_bytes[:target_size - 2] + b'\xff\xd9'
    
    ffd9_pos = jpeg_bytes.rfind(b'\xff\xd9')
    if ffd9_pos == -1:
        return jpeg_bytes + b'\x00' * (target_size - len(jpeg_bytes))
    
    padding_needed = target_size - len(jpeg_bytes)
    pad_data = bytearray()
    remaining = padding_needed
    
    while remaining > 0:
        if remaining >= 4:
            chunk = min(remaining - 4, 65533)
            pad_data += b'\xff\xfe' + struct.pack('>H', chunk + 2) + b'\x00' * chunk
            remaining -= (4 + chunk)
        else:
            pad_data += b'\x00' * remaining
            remaining = 0
    
    return jpeg_bytes[:ffd9_pos] + bytes(pad_data) + jpeg_bytes[ffd9_pos:]


class TemplateParser:
    """
    Parses the Windows PCAP binary stream to extract JPEG slot metadata.
    Each slot records: (absolute_offset_in_stream, jpeg_size, tile_w, tile_h)
    """
    
    def __init__(self, template_path):
        with open(template_path, "rb") as f:
            self.template = bytearray(f.read())
        
        self.slots = []  # (jpeg_abs_offset, jpeg_size, w, h)
        self._parse()
        
        # Group slots by (w, h) to know which unique tile sizes we need
        self.unique_sizes = set((w, h) for _, _, w, h in self.slots)
        print(f"[*] Template: {len(self.template)} bytes, "
              f"{len(self.slots)} JPEG slots, "
              f"{len(self.unique_sizes)} unique sizes")
    
    def _parse(self):
        pos = 0
        while pos < len(self.template):
            if self.template[pos:pos+8] != b'EPRD0600':
                break
            
            fb = self.template[pos+20]
            if fb == 0xCC:
                ps = struct.unpack('<I', self.template[pos+16:pos+20])[0]
                pos += 20 + ps
                continue
            
            ps = struct.unpack('>I', self.template[pos+16:pos+20])[0]
            payload_start = pos + 20
            tp = 4  # skip frame_type
            
            while tp + 16 <= ps:
                x, y, w, h = struct.unpack('>HHHH',
                    self.template[payload_start+tp:payload_start+tp+8])
                if w == 0 or h == 0 or w > 2000:
                    break
                tp += 16
                
                abs_jpeg_start = payload_start + tp
                if self.template[abs_jpeg_start:abs_jpeg_start+2] != b'\xff\xd8':
                    break
                
                je = self.template.find(b'\xff\xd9', abs_jpeg_start) + 2
                jpeg_size = je - abs_jpeg_start
                self.slots.append((abs_jpeg_start, jpeg_size, w, h))
                tp = je - payload_start
            
            pos += 20 + ps
    
    def swap_jpeg(self, slot_idx, new_jpeg_padded):
        """Write padded JPEG bytes directly into the template buffer."""
        offset, size, w, h = self.slots[slot_idx]
        assert len(new_jpeg_padded) == size, \
            f"JPEG size mismatch: {len(new_jpeg_padded)} vs {size}"
        self.template[offset:offset + size] = new_jpeg_padded


class VideoStreamer:
    """
    Template-swap video streamer for Epson projectors.
    
    Uses the raw Windows PCAP binary as a carrier template. For each frame:
    1. Captures the screen with grim (Wayland) or mss (X11)
    2. Encodes unique tile sizes as Pillow JPEG
    3. Pads/truncates to match template slot sizes
    4. Swaps JPEG data into the template buffer
    5. Sends the full template through the 1460B TCP chunker
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
        
        # Load and parse the template
        if not os.path.exists(TEMPLATE_FILE):
            raise FileNotFoundError(
                f"Windows template not found: {TEMPLATE_FILE}\n"
                "This file is required for the template-swap streaming approach.")
        
        self.tpl = TemplateParser(TEMPLATE_FILE)
        
        # Pre-compute: for each unique (w,h), which slot indices use it
        self._size_to_slots = {}
        for idx, (offset, size, w, h) in enumerate(self.tpl.slots):
            key = (w, h)
            self._size_to_slots.setdefault(key, []).append(idx)
        
        # Determine capture method
        self.capture_method = None
        
        is_wayland = os.environ.get('XDG_SESSION_TYPE') == 'wayland' or \
                     os.environ.get('WAYLAND_DISPLAY') is not None
        
        if is_wayland:
            if self._check_grim():
                self.capture_method = self._capture_grim
                print(f"[+] Wayland detected. Using 'grim' for screen capture.")
            else:
                print("[-] Wayland detected but 'grim' not found!")
                print("    Install it: sudo pacman -S grim")
        else:
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
        try:
            result = subprocess.run(['which', 'grim'], stdout=subprocess.DEVNULL,
                                   stderr=subprocess.DEVNULL)
            return result.returncode == 0
        except Exception:
            return False
    
    def _try_mss(self):
        try:
            import mss
            self.sct = mss.mss()
            for idx, m in enumerate(self.sct.monitors[1:], 1):
                try:
                    self.sct.grab(m)
                    self.monitor = m
                    return True
                except Exception:
                    pass
        except Exception:
            pass
        return False

    def _capture_grim(self):
        """Capture screen using grim, return PIL Image scaled to VNC viewport."""
        proc = subprocess.run(
            ['grim', '-t', 'png', '-'],
            capture_output=True
        )
        if proc.returncode != 0:
            return None
        img = Image.open(io.BytesIO(proc.stdout))
        return img.resize((self.stream_width, self.stream_height), Image.LANCZOS)

    def _capture_mss(self):
        import numpy as np
        sct_img = self.sct.grab(self.monitor)
        raw = np.array(sct_img)
        img = Image.fromarray(raw[:, :, [2, 1, 0]], "RGB")
        return img.resize((self.stream_width, self.stream_height), Image.LANCZOS)

    def _encode_tile(self, full_image, w, h, x_off=0, y_off=0):
        """Crop and encode a single tile as JPEG."""
        tile_img = full_image.crop((x_off, y_off, x_off + w, y_off + h))
        buf = io.BytesIO()
        tile_img.save(buf, format="JPEG", quality=self.jpeg_quality,
                     subsampling=2, optimize=False, progressive=False)
        return buf.getvalue()

    def _swap_frame_into_template(self, full_image):
        """
        Encode the captured frame into JPEG tiles and swap them into
        ALL 233 slots of the template buffer.
        
        For each unique tile (w, h), we encode once and pad to each slot's
        required size, then write into the template.
        """
        # For each unique (w, h), encode the tile from the image
        jpeg_cache = {}  # (w, h) -> raw jpeg bytes
        
        for (w, h) in self.tpl.unique_sizes:
            # Find where this tile sits in the VNC viewport
            # Use (0,0) as base for full-frame tiles, actual coordinates
            # come from the slot's region descriptor in the template
            # For simplicity, we crop at (0,0) for any tile regardless of slot position
            # since the full image is our captured desktop
            pass
        
        # Actually, each slot has its own (x, y, w, h) from the region descriptor.
        # Different slots with the same (w,h) might reference different coordinates.
        # We need to encode per-slot, not per-size.
        # BUT: for performance, we can cache by (x_crop, y_crop, w, h).
        
        # Read the actual region descriptors from the template to get crop coords
        for idx, (offset, jpeg_size, w, h) in enumerate(self.tpl.slots):
            # The region descriptor is 16 bytes BEFORE the JPEG data:
            # x(2) + y(2) + w(2) + h(2) + flags(4) + ts(4)
            region_offset = offset - 16
            rx, ry = struct.unpack('>HH',
                self.tpl.template[region_offset:region_offset+4])
            
            cache_key = (rx, ry, w, h)
            if cache_key not in jpeg_cache:
                # Crop from the full image at the region's coordinates
                # Clamp to image bounds
                crop_x = min(rx, self.stream_width - 1)
                crop_y = min(ry, self.stream_height - 1)
                crop_w = min(w, self.stream_width - crop_x)
                crop_h = min(h, self.stream_height - crop_y)
                
                if crop_w > 0 and crop_h > 0:
                    jpeg_raw = self._encode_tile(full_image, crop_w, crop_h,
                                                 crop_x, crop_y)
                else:
                    # Fallback: encode a black tile
                    jpeg_raw = self._encode_tile(
                        Image.new('RGB', (w, h), (0, 0, 0)), w, h)
                
                jpeg_cache[cache_key] = jpeg_raw
            
            # Pad to exact slot size
            padded = pad_jpeg(jpeg_cache[cache_key], jpeg_size)
            self.tpl.swap_jpeg(idx, padded)
    
    def _send_template(self):
        """Send the full template through the TCP chunker."""
        data = bytes(self.tpl.template)
        for i in range(0, len(data), 1460):
            self.client.s_video.sendall(data[i:i + 1460])
            time.sleep(0.002)

    def start_streaming(self):
        """
        Continuous streaming loop using template-swap approach.
        
        Each iteration:
        1. Captures the screen
        2. Swaps JPEG tiles into the template
        3. Sends the full 3.2MB template (~4.5s at 730KB/s)
        """
        print(f"\n[*] Starting template-swap video stream...")
        print(f"[*] Template: {len(self.tpl.template)} bytes, "
              f"{len(self.tpl.slots)} JPEG slots")
        print(f"[*] Viewport: {self.stream_width}x{self.stream_height}")
        print(f"[*] Estimated frame time: ~{len(self.tpl.template) / 730000:.1f}s")
        
        use_test_card = self.capture_method is None
        if use_test_card:
            print("[!] No screen capture available. Using test gradient frame.")
        
        try:
            while True:
                t0 = time.time()
                
                # 1. Capture
                if use_test_card:
                    frame = Image.new('RGB',
                        (self.stream_width, self.stream_height),
                        color=(40, 80, 160))
                else:
                    frame = self.capture_method()
                    if frame is None:
                        print("[-] Capture failed, retrying...")
                        time.sleep(0.1)
                        continue
                
                t_capture = time.time()
                
                # 2. Swap tiles into template
                self._swap_frame_into_template(frame)
                t_encode = time.time()
                
                # 3. Send the full template
                self._send_template()
                t_send = time.time()
                
                self.frame_idx += 1
                
                if self.frame_idx <= 3 or self.frame_idx % 10 == 0:
                    print(f"  Frame {self.frame_idx}: "
                          f"capture={t_capture-t0:.2f}s "
                          f"encode={t_encode-t_capture:.2f}s "
                          f"send={t_send-t_encode:.2f}s "
                          f"total={t_send-t0:.2f}s")
                
        except KeyboardInterrupt:
            print("\n[*] Stopping video stream...")
        except ConnectionResetError as e:
            print(f"\n[-] Connection reset: {e}")
        except BrokenPipeError as e:
            print(f"\n[-] Broken pipe: {e}")
        except Exception as e:
            print(f"[-] Video streaming error: {e}")
            import traceback
            traceback.print_exc()