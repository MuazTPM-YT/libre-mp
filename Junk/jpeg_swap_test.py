#!/usr/bin/env python3
"""
JPEG SWAP TEST: Take raw Windows stream, replace ONLY the JPEG data 
(not timestamps, not region headers, not EPRD headers).
Swap in solid RED Pillow JPEGs of SAME SIZE (padded to match).

If works → timestamps aren't checksums of JPEG data, they're just timestamps
If fails → timestamps ARE derived from JPEG data somehow
"""
import struct, socket, time, io
from PIL import Image

def pad_jpeg(jpeg_bytes, target_size):
    """Pad JPEG to exact size by adding padding before FFD9 marker."""
    if len(jpeg_bytes) >= target_size:
        return jpeg_bytes[:target_size]
    
    ffd9_pos = jpeg_bytes.rfind(b'\xff\xd9')
    if ffd9_pos == -1:
        return jpeg_bytes + b'\x00' * (target_size - len(jpeg_bytes))
    
    padding_needed = target_size - len(jpeg_bytes)
    pad_data = bytearray()
    remaining = padding_needed
    
    while remaining > 0:
        if remaining >= 4:
            # COM marker: FF FE + 2-byte length + data
            chunk = min(remaining - 4, 65533)  # max COM payload = 65533
            pad_data += b'\xff\xfe' + struct.pack('>H', chunk + 2) + b'\x00' * chunk
            remaining -= (4 + chunk)
        else:
            pad_data += b'\x00' * remaining
            remaining = 0
    
    return jpeg_bytes[:ffd9_pos] + bytes(pad_data) + jpeg_bytes[ffd9_pos:]

def create_jpeg(w, h, color=(255, 0, 0), quality=85):
    img = Image.new('RGB', (w, h), color)
    buf = io.BytesIO()
    img.save(buf, format='JPEG', quality=quality, subsampling='4:2:0')
    return buf.getvalue()

def main():
    with open("windows_perfect_stream.bin", "rb") as f:
        raw = bytearray(f.read())
    
    # Parse all blocks and find JPEG data positions
    pos = 0
    jpeg_regions = []  # (abs_offset, size, tile_w, tile_h)
    while pos < len(raw):
        if raw[pos:pos+8] != b'EPRD0600': break
        fb = raw[pos+20]
        if fb == 0xCC:
            ps = struct.unpack('<I', raw[pos+16:pos+20])[0]
            pos += 20 + ps; continue
        
        ps = struct.unpack('>I', raw[pos+16:pos+20])[0]
        payload_start = pos + 20
        tp = 4
        while tp+16 <= ps:
            x,y,w,h = struct.unpack('>HHHH', raw[payload_start+tp:payload_start+tp+8])
            if w==0 or h==0 or w>2000: break
            tp += 16
            abs_jpeg_start = payload_start + tp
            if raw[abs_jpeg_start:abs_jpeg_start+2] != b'\xff\xd8': break
            je = raw.find(b'\xff\xd9', abs_jpeg_start) + 2
            jpeg_size = je - abs_jpeg_start
            jpeg_regions.append((abs_jpeg_start, jpeg_size, w, h))
            tp = je - payload_start
        
        pos += 20 + ps
    
    print(f"Found {len(jpeg_regions)} JPEG regions to swap")
    
    # Create replacement JPEGs
    modified = bytearray(raw)
    for i, (offset, orig_size, w, h) in enumerate(jpeg_regions):
        new_jpeg = create_jpeg(w, h, color=(255, 0, 0))  # RED
        padded = pad_jpeg(new_jpeg, orig_size)
        
        if len(padded) != orig_size:
            print(f"  WARNING: tile {i} size mismatch: {len(padded)} vs {orig_size}")
            continue
        
        modified[offset:offset+orig_size] = padded
        
        if i < 4:
            print(f"  Tile {i}: ({w}x{h}) swapped {orig_size}B")
    
    # Verify size unchanged
    print(f"\nOriginal size: {len(raw)}")
    print(f"Modified size: {len(modified)}")
    
    # Count how many bytes changed
    changes = sum(1 for i in range(len(raw)) if raw[i] != modified[i])
    print(f"Bytes changed: {changes}")
    
    # Save for replay
    with open("jpeg_swapped_stream.bin", "wb") as f:
        f.write(bytes(modified))
    print("Saved jpeg_swapped_stream.bin")
    
    # Connect and send
    from epson_projector.client import EpsonEasyMPClient
    from epson_projector.wifi import interactive_wifi_setup, revert_wifi
    
    print("\n--- Wi-Fi Setup ---")
    connected, orig_uuid, proj_ssid_to_delete = interactive_wifi_setup()
    
    try:
        client = EpsonEasyMPClient()
        success = client.connect_and_negotiate()
        if not success: return
        
        print(f"\n[1] Raw Windows UNMODIFIED (control)")
        print(f"[2] JPEG-swapped (RED, same sizes, original timestamps)")
        choice = input("(1 or 2): ").strip()
        
        data = bytes(raw) if choice == "1" else bytes(modified)
        label = "UNMODIFIED" if choice == "1" else "JPEG-SWAPPED"
        
        print(f"\nSending {label}: {len(data)} bytes")
        chunks = 0
        try:
            for i in range(0, len(data), 1460):
                client.s_video.sendall(data[i:i+1460])
                time.sleep(0.002)
                chunks += 1
            print(f"[+] {chunks} chunks, no RST!")
            print("[!] CHECK THE WALL — is it RED?")
            while True:
                time.sleep(1)
        except (ConnectionResetError, BrokenPipeError) as e:
            print(f"[-] {e} at chunk {chunks}")
        
        client.disconnect()
    except KeyboardInterrupt: pass
    except Exception as e:
        import traceback; traceback.print_exc()
    finally:
        print("\n--- Clean Up ---")
        revert_wifi(orig_uuid, proj_ssid_to_delete)

if __name__ == "__main__":
    main()
