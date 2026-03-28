#!/usr/bin/env python3
"""
SUSTAINED PILLOW JPEG TEST

Build 50 continuous frames of Pillow-encoded JPEGs (solid bright color)
and send as one monolithic stream. This tests both:
1. Whether the projector needs sustained frames to lock on
2. Whether Pillow-encoded JPEGs are compatible with the hardware decoder

The stream structure matches the working Windows replay exactly:
  META block (once) → [EPRD JPEG header + frame_type(4) + 4 tiles] × 50
"""
import struct
import socket
import time
import io

WIN_BIN = "windows_perfect_stream.bin"

from epson_projector import config

TILE_GRID = config.TILE_GRID  # [(0,0,624,416), (624,0,400,416), (0,416,624,352), (624,416,400,352)]

def create_pillow_jpeg(w, h, color=(255, 0, 0), quality=50):
    """Create a solid-color JPEG using Pillow."""
    from PIL import Image
    img = Image.new('RGB', (w, h), color)
    buf = io.BytesIO()
    img.save(buf, format='JPEG', quality=quality, subsampling='4:2:0',
             optimize=False, progressive=False)
    return buf.getvalue()

def create_windows_jpeg(tile_idx=0):
    """Extract a real Windows JPEG from the binary for comparison."""
    with open(WIN_BIN, "rb") as f:
        data = f.read()
    
    meta_end = 66
    pos = meta_end + 20 + 4  # skip JPEG EPRD header + frame_type
    for i in range(tile_idx + 1):
        x, y, w, h = struct.unpack('>HHHH', data[pos:pos+8])
        pos += 16
        je = data.find(b'\xff\xd9', pos) + 2
        if i == tile_idx:
            return data[pos:je], (x, y, w, h)
        pos = je

def build_meta_block(ip):
    meta = bytes.fromhex("cc0000000400030020200001ff00ff00ff0010080000000006400384000000600400024000000000000000000000")
    hdr = b'EPRD0600' + ip + struct.pack('<II', 0, len(meta))
    return hdr + meta

def build_stream(ip, tiles_jpeg, num_frames=50):
    """Build a monolithic stream of num_frames frames."""
    frame_type = struct.pack('>I', 4)
    
    buf = bytearray()
    
    # Meta block (once)
    buf += build_meta_block(ip)
    
    for f in range(num_frames):
        # Build tile blobs
        tile_data = bytearray()
        for i, (x, y, w, h) in enumerate(TILE_GRID):
            ts = int(time.time() * 1000) & 0xFFFFFFFF
            region = struct.pack('>HHHH', x, y, w, h) + struct.pack('>II', 0x00000007, ts)
            tile_data += region + tiles_jpeg[i]
        
        payload_size = len(frame_type) + len(tile_data)
        jpeg_hdr = b'EPRD0600' + ip + struct.pack('>II', 0, payload_size)
        buf += jpeg_hdr + frame_type + tile_data
    
    return bytes(buf)

def main():
    from epson_projector.client import EpsonEasyMPClient
    from epson_projector.wifi import interactive_wifi_setup, revert_wifi
    
    print("=== SUSTAINED PILLOW JPEG TEST ===\n")
    
    # Create Pillow JPEGs for each tile (bright red)
    print("[*] Generating Pillow JPEGs (solid RED)...")
    pillow_jpegs = []
    for i, (x, y, w, h) in enumerate(TILE_GRID):
        jpeg = create_pillow_jpeg(w, h, color=(255, 0, 0))
        pillow_jpegs.append(jpeg)
        print(f"  Tile {i}: ({x},{y}) {w}x{h} → {len(jpeg)} bytes")
    
    # Also create Windows JPEGs for comparison
    print("\n[*] Extracting Windows JPEGs from binary...")
    with open(WIN_BIN, "rb") as f:
        data = f.read()
    meta_end = 66
    pos = meta_end + 20 + 4
    win_jpegs = []
    for i in range(4):
        x, y, w, h = struct.unpack('>HHHH', data[pos:pos+8])
        pos += 16
        je = data.find(b'\xff\xd9', pos) + 2
        win_jpegs.append(data[pos:je])
        print(f"  Tile {i}: ({x},{y}) {w}x{h} → {len(data[pos:je])} bytes")
        pos = je
    
    ip = socket.inet_aton("192.168.88.2")
    
    NUM_FRAMES = 50
    
    # Pre-build both streams
    print(f"\n[*] Pre-building {NUM_FRAMES}-frame streams...")
    pillow_stream = build_stream(ip, pillow_jpegs, NUM_FRAMES)
    windows_stream = build_stream(ip, win_jpegs, NUM_FRAMES)
    print(f"  Pillow stream: {len(pillow_stream)} bytes ({len(pillow_stream)//1460+1} chunks)")
    print(f"  Windows stream: {len(windows_stream)} bytes ({len(windows_stream)//1460+1} chunks)")
    
    # Connect
    print("\n--- Wi-Fi Setup ---")
    connected, orig_uuid, proj_ssid_to_delete = interactive_wifi_setup()
    
    try:
        client = EpsonEasyMPClient()
        success = client.connect_and_negotiate()
        if not success: return
        actual_ip = socket.inet_aton(client.my_ip)
        
        # Patch IPs if needed
        if actual_ip != ip:
            pillow_stream = pillow_stream.replace(ip, actual_ip)
            windows_stream = windows_stream.replace(ip, actual_ip)
        
        print(f"\nWhich JPEG source?")
        print(f"[1] Pillow JPEGs (solid RED) — {NUM_FRAMES} frames")
        print(f"[2] Windows JPEGs — {NUM_FRAMES} frames")
        print(f"[3] Full raw Windows replay (control)")
        choice = input("(1-3): ").strip()
        
        if choice == "3":
            stream = data  # full 3.2MB raw
            if actual_ip != ip:
                stream = stream.replace(ip, actual_ip)
            label = "FULL raw Windows replay"
        elif choice == "2":
            stream = windows_stream
            label = f"Windows JPEGs × {NUM_FRAMES} frames"
        else:
            stream = pillow_stream
            label = f"Pillow JPEGs × {NUM_FRAMES} frames"
        
        print(f"\n[*] Sending: {label} ({len(stream)} bytes)")
        
        chunks_sent = 0
        t0 = time.time()
        try:
            for i in range(0, len(stream), 1460):
                client.s_video.sendall(stream[i:i+1460])
                time.sleep(0.002)
                chunks_sent += 1
            
            elapsed = time.time() - t0
            print(f"[+] {chunks_sent} chunks sent in {elapsed:.1f}s, no RST!")
            print(f"[!] CHECK THE WALL!")
            
            for i in range(20):
                client._send_frame_keepalive()
                time.sleep(1)
                
        except (ConnectionResetError, BrokenPipeError) as e:
            print(f"[-] {e} at chunk {chunks_sent}")
        
        client.disconnect()
    except KeyboardInterrupt: pass
    except Exception as e:
        import traceback; traceback.print_exc()
    finally:
        print("\n--- Clean Up ---")
        revert_wifi(orig_uuid, proj_ssid_to_delete)

if __name__ == "__main__":
    main()
