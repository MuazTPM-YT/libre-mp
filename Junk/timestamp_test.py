#!/usr/bin/env python3
"""
TIMESTAMP TEST: The only difference between our Python batch 1 and Windows batch 1
is the timestamp values. This test sends Python batch 0+1 with Windows timestamps.
"""
import struct
import socket
import time

WIN_BIN = "windows_perfect_stream.bin"

def main():
    with open(WIN_BIN, "rb") as f:
        data = f.read()
    
    ip = socket.inet_aton("192.168.88.2")
    
    # Parse Windows batch boundaries
    meta_size = struct.unpack('<I', data[16:20])[0]  # 46
    meta_end = 20 + meta_size  # 66
    b1_size = struct.unpack('>I', data[meta_end+16:meta_end+20])[0]
    b1_end = meta_end + 20 + b1_size  # 76619
    b2_size = struct.unpack('>I', data[b1_end+16:b1_end+20])[0]
    b2_end = b1_end + 20 + b2_size
    
    # Extract batch 1 tiles WITH their original Windows timestamps
    pos = meta_end + 20 + 4  # skip EPRD header + frame_type
    tiles_b1_with_ts = []
    for i in range(4):
        x, y, w, h = struct.unpack('>HHHH', data[pos:pos+8])
        flags, ts = struct.unpack('>II', data[pos+8:pos+16])
        pos += 16
        je = data.find(b'\xff\xd9', pos) + 2
        jpeg = data[pos:je]
        tiles_b1_with_ts.append((x, y, w, h, flags, ts, jpeg))
        print(f"Tile {i}: ({x},{y}) {w}x{h} flags=0x{flags:08x} ts=0x{ts:08x} JPEG={len(jpeg)}B")
        pos = je
    
    # Extract batch 2 tiles
    b2_pos = b1_end + 20 + 4
    tiles_b2 = []
    for i in range(4):
        x, y, w, h = struct.unpack('>HHHH', data[b2_pos:b2_pos+8])
        b2_pos += 16
        je = data.find(b'\xff\xd9', b2_pos) + 2
        tiles_b2.append((x, y, w, h, data[b2_pos:je]))
        b2_pos = je
    
    # Build Python meta (hardcoded = same as Windows)
    meta = bytes.fromhex("cc0000000400030020200001ff00ff00ff0010080000000006400384000000600400024000000000000000000000")
    meta_hdr = b'EPRD0600' + ip + struct.pack('<II', 0, len(meta))
    
    # Build Python batch 1 with WINDOWS timestamps
    frame_type = struct.pack('>I', 4)
    tile_data_win_ts = bytearray()
    for (x, y, w, h, flags, ts, jpeg) in tiles_b1_with_ts:
        region = struct.pack('>HHHH', x, y, w, h) + struct.pack('>II', flags, ts)
        tile_data_win_ts += region + jpeg
    
    ps1 = len(frame_type) + len(tile_data_win_ts)
    jpeg_hdr_1 = b'EPRD0600' + ip + struct.pack('>II', 0, ps1)
    
    py_batch1_win_ts = jpeg_hdr_1 + frame_type + bytes(tile_data_win_ts)
    
    # Build Python batch 1 with OUR timestamps (time.time)
    tile_data_our_ts = bytearray()
    for (x, y, w, h, flags, ts, jpeg) in tiles_b1_with_ts:
        our_ts = int(time.time() * 1000) & 0xFFFFFFFF
        region = struct.pack('>HHHH', x, y, w, h) + struct.pack('>II', flags, our_ts)
        tile_data_our_ts += region + jpeg
    
    py_batch1_our_ts = jpeg_hdr_1 + frame_type + bytes(tile_data_our_ts)
    
    # Build Python batch 2 with time.time() timestamps
    tile_data_b2 = bytearray()
    for (x, y, w, h, jpeg) in tiles_b2:
        ts = int(time.time() * 1000) & 0xFFFFFFFF
        region = struct.pack('>HHHH', x, y, w, h) + struct.pack('>II', 0x00000007, ts)
        tile_data_b2 += region + jpeg
    ps2 = len(frame_type) + len(tile_data_b2)
    jpeg_hdr_2 = b'EPRD0600' + ip + struct.pack('>II', 0, ps2)
    py_batch2 = jpeg_hdr_2 + frame_type + bytes(tile_data_b2)
    
    # Verify Python batch 1 with Windows timestamps matches raw
    raw_batch1 = data[meta_end:b1_end]
    print(f"\nPython batch 1 (win ts) matches raw: {py_batch1_win_ts == raw_batch1}")
    print(f"Python batch 1 (our ts) matches raw: {py_batch1_our_ts == raw_batch1}")
    
    # Streams to test
    stream_win_ts = (meta_hdr + meta) + py_batch1_win_ts + py_batch2
    stream_our_ts = (meta_hdr + meta) + py_batch1_our_ts + py_batch2
    
    print(f"\nStream with Windows timestamps: {len(stream_win_ts)} bytes")
    print(f"Stream with our timestamps:     {len(stream_our_ts)} bytes")
    
    # Compare the two meta blocks
    raw_meta_block = data[:meta_end]
    our_meta_block = meta_hdr + meta
    print(f"\nMeta block matches raw: {our_meta_block == raw_meta_block}")
    if our_meta_block != raw_meta_block:
        for i in range(min(len(our_meta_block), len(raw_meta_block))):
            if our_meta_block[i] != raw_meta_block[i]:
                print(f"  DIFF at byte {i}: raw=0x{raw_meta_block[i]:02x} ours=0x{our_meta_block[i]:02x}")
    
    from epson_projector.client import EpsonEasyMPClient
    from epson_projector.wifi import interactive_wifi_setup, revert_wifi
    
    print("\n--- Wi-Fi Setup ---")
    connected, orig_uuid, proj_ssid_to_delete = interactive_wifi_setup()
    
    try:
        client = EpsonEasyMPClient()
        success = client.connect_and_negotiate()
        if not success: return
        actual_ip = socket.inet_aton(client.my_ip)
        
        # Patch IP if needed
        if actual_ip != ip:
            stream_win_ts = stream_win_ts.replace(ip, actual_ip)
            stream_our_ts = stream_our_ts.replace(ip, actual_ip)
        
        print(f"\nWhich test?")
        print("[1] Python batch 1 with WINDOWS timestamps + Python batch 2")
        print("[2] Python batch 1 with OUR timestamps + Python batch 2")
        choice = input("(1 or 2): ").strip()
        
        stream = stream_win_ts if choice == "1" else stream_our_ts
        label = "WINDOWS timestamps" if choice == "1" else "OUR timestamps"
        
        print(f"\nSending Python batch 0+1 ({label}) + batch 2: {len(stream)} bytes")
        
        chunks_sent = 0
        try:
            for i in range(0, len(stream), 1460):
                client.s_video.sendall(stream[i:i+1460])
                time.sleep(0.002)
                chunks_sent += 1
            
            print(f"[+] SUCCESS! {chunks_sent} chunks, no RST!")
            for i in range(10):
                client._send_frame_keepalive(); time.sleep(1)
        except ConnectionResetError:
            print(f"[-] RST after {chunks_sent} chunks!")
        
        client.disconnect()
    except KeyboardInterrupt:
        pass
    except Exception as e:
        print(f"[-] {e}")
        import traceback; traceback.print_exc()
    finally:
        print("\n--- Clean Up ---")
        revert_wifi(orig_uuid, proj_ssid_to_delete)

if __name__ == "__main__":
    main()
