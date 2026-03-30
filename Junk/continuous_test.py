#!/usr/bin/env python3
"""
CONTINUOUS STREAM TEST: Pre-assemble ALL frames into a single continuous
byte buffer BEFORE sending, then push through the 1460-byte chunker
with 2ms delays — EXACTLY like the working Windows replay does.

This tests whether the inter-frame gap causes the RST.
"""
import struct
import socket
import time

WIN_BIN = "windows_perfect_stream.bin"
MY_IP_PLACEHOLDER = "192.168.88.2"

def build_eprd_meta_header(ip_bytes, meta_size):
    return b'EPRD0600' + ip_bytes + struct.pack('<II', 0, meta_size)

def build_eprd_jpeg_header(ip_bytes, jpeg_size):
    return b'EPRD0600' + ip_bytes + struct.pack('>II', 0, jpeg_size)

def extract_windows_tiles(data):
    """Extract the 4 tile JPEGs from the first batch."""
    meta_size = struct.unpack('<I', data[16:20])[0]
    meta_end = 20 + meta_size
    batch_start = meta_end + 20
    
    pos = batch_start + 4  # skip frame_type
    tiles = []
    for i in range(4):
        x, y, w, h = struct.unpack('>HHHH', data[pos:pos+8])
        pos += 16
        je = data.find(b'\xff\xd9', pos) + 2
        jpeg = data[pos:je]
        tiles.append((x, y, w, h, jpeg))
        pos = je
    return tiles

def build_continuous_stream(tiles, ip_bytes, num_frames=10):
    """
    Pre-assemble ALL frames into a single contiguous byte buffer.
    Frame 1 has meta block, frames 2+ don't. All have frame_type.
    """
    meta = bytes.fromhex("cc0000000400030020200001ff00ff00ff0010080000000006400384000000600400024000000000000000000000")
    frame_type = struct.pack('>I', 4)
    
    buf = bytearray()
    
    for frame_num in range(num_frames):
        # Build tile blobs for this frame
        tile_blobs = bytearray()
        for (x, y, w, h, jpeg) in tiles:
            ts = int(time.time() * 1000) & 0xFFFFFFFF
            region = struct.pack('>HHHH', x, y, w, h) + struct.pack('>II', 0x00000007, ts)
            tile_blobs += region + jpeg
        
        jpeg_payload_size = len(frame_type) + len(tile_blobs)
        jpeg_hdr = build_eprd_jpeg_header(ip_bytes, jpeg_payload_size)
        
        if frame_num == 0:
            # First frame: meta block + jpeg batch
            meta_hdr = build_eprd_meta_header(ip_bytes, len(meta))
            buf += meta_hdr + meta
        
        buf += jpeg_hdr + frame_type + tile_blobs
    
    return bytes(buf)

def main():
    from epson_projector.client import EpsonEasyMPClient
    from epson_projector.wifi import interactive_wifi_setup, revert_wifi
    
    print("=== CONTINUOUS STREAM TEST ===\n")
    
    with open(WIN_BIN, "rb") as f:
        win_data = f.read()
    
    tiles = extract_windows_tiles(win_data)
    print(f"[*] Extracted {len(tiles)} Windows tiles")
    for i, (x, y, w, h, jpeg) in enumerate(tiles):
        print(f"    Tile {i}: ({x},{y}) {w}x{h} JPEG={len(jpeg)}B")
    
    NUM_FRAMES = 10
    
    # Pre-build the stream using a placeholder IP (we'll patch it later)
    ip_bytes = socket.inet_aton(MY_IP_PLACEHOLDER)
    stream = build_continuous_stream(tiles, ip_bytes, NUM_FRAMES)
    print(f"\n[*] Pre-assembled {NUM_FRAMES} frames = {len(stream)} bytes")
    
    # Wi-Fi
    print("\n--- Wi-Fi Setup ---")
    connected, orig_uuid, proj_ssid_to_delete = interactive_wifi_setup()
    
    try:
        print("\n--- Connecting ---")
        client = EpsonEasyMPClient()
        success = client.connect_and_negotiate()
        
        if success:
            # Patch IP if different
            actual_ip = socket.inet_aton(client.my_ip)
            if actual_ip != ip_bytes:
                stream = stream.replace(ip_bytes, actual_ip)
                print(f"[*] Patched IP to {client.my_ip}")
            
            print(f"\n[*] Sending {NUM_FRAMES} frames as CONTINUOUS stream...")
            print(f"[*] Total: {len(stream)} bytes in {len(stream)//1460 + 1} chunks")
            
            chunk_size = 1460
            chunks_sent = 0
            for i in range(0, len(stream), chunk_size):
                chunk = stream[i:i+chunk_size]
                client.s_video.sendall(chunk)
                time.sleep(0.002)
                chunks_sent += 1
            
            print(f"[+] ALL {chunks_sent} chunks sent successfully!")
            print(f"[!] LOOK AT THE WALL! Do you see the Windows desktop?")
            print(f"[*] Holding connection for 30s...")
            
            for i in range(30):
                client._send_frame_keepalive()
                time.sleep(1)
        
        client.disconnect()
        
    except KeyboardInterrupt:
        print("\n[*] Exiting...")
        try: client.disconnect()
        except: pass
    except Exception as e:
        print(f"[-] Error: {e}")
        import traceback
        traceback.print_exc()
    finally:
        print("\n--- Clean Up ---")
        revert_wifi(orig_uuid, proj_ssid_to_delete)

if __name__ == "__main__":
    main()
