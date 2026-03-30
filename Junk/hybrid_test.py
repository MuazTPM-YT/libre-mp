#!/usr/bin/env python3
"""
HYBRID TEST: Extract the REAL Windows JPEGs from the PCAP binary,
wrap them in our Python-built EPRD protocol headers, and send them
through send_video_batch().

This definitively isolates JPEG content vs. protocol issues:
- If this works (image shows, no RST) → Pillow JPEG encoding is the problem
- If this fails (RST) → Protocol headers/timing is still wrong
"""
import struct
import time

def extract_windows_tiles(bin_path):
    """Extract the 4 tile JPEGs from the first batch in windows_perfect_stream.bin."""
    with open(bin_path, "rb") as f:
        data = f.read()
    
    # Skip META block: 20-byte EPRD header + 46-byte meta
    meta_size = struct.unpack('<I', data[16:20])[0]  # 46
    meta_end = 20 + meta_size  # offset 66
    
    # JPEG batch header
    jpeg_batch_size = struct.unpack('>I', data[meta_end+16:meta_end+20])[0]
    batch_start = meta_end + 20  # offset 86
    
    # Parse tiles (skip 4-byte frame_type)
    pos = batch_start + 4  # skip 0x00000004
    tiles = []
    for i in range(4):
        x, y, w, h = struct.unpack('>HHHH', data[pos:pos+8])
        pos += 16  # skip full region descriptor (16 bytes)
        
        # Find JPEG end
        je = data.find(b'\xff\xd9', pos) + 2
        jpeg = data[pos:je]
        tiles.append((x, y, w, h, jpeg))
        pos = je
        print(f"  Windows Tile {i}: X={x} Y={y} W={w}x{h} JPEG={len(jpeg)}B")
    
    return tiles

def main():
    from epson_projector.client import EpsonEasyMPClient
    from epson_projector.wifi import interactive_wifi_setup, revert_wifi
    
    print("=== HYBRID TEST: Windows JPEGs + Python Protocol ===\n")
    
    # Extract Windows tiles
    print("[*] Extracting Windows JPEGs from windows_perfect_stream.bin...")
    win_tiles = extract_windows_tiles("windows_perfect_stream.bin")
    
    print(f"\n[*] Extracted {len(win_tiles)} tiles. Total JPEG: "
          f"{sum(len(t[4]) for t in win_tiles)} bytes\n")
    
    # Connect
    print("--- Wi-Fi Setup ---")
    connected, orig_uuid, proj_ssid_to_delete = interactive_wifi_setup()
    
    try:
        print("\n--- Initializing Projector Client ---")
        client = EpsonEasyMPClient()
        success = client.connect_and_negotiate()
        
        if success:
            print("\n[*] === HYBRID TEST: Sending Windows JPEGs via Python protocol ===")
            
            # Send the same Windows tiles 5 times to test multi-frame
            for frame_num in range(5):
                print(f"\n[*] Sending frame {frame_num + 1}/5...")
                ok = client.send_video_batch(win_tiles)
                if not ok:
                    print(f"[-] FAILED on frame {frame_num + 1}!")
                    break
                print(f"[+] Frame {frame_num + 1} accepted!")
                
                # Small delay between frames (similar to screen capture time)
                time.sleep(0.1)
            
            if ok:
                print("\n[+] ALL 5 FRAMES SENT SUCCESSFULLY!")
                print("[!] Look at the projector wall — do you see the Windows desktop?")
                print("[*] Holding connection for 30 seconds...")
                for i in range(30):
                    client._send_frame_keepalive()
                    time.sleep(1)
            
        client.disconnect()
        
    except KeyboardInterrupt:
        print("\n[*] Exiting...")
        try: client.disconnect()
        except: pass
    finally:
        print("\n--- Clean Up ---")
        revert_wifi(orig_uuid, proj_ssid_to_delete)

if __name__ == "__main__":
    main()
