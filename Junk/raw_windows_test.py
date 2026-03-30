#!/usr/bin/env python3
"""
DEFINITIVE TEST: Send just the first 2 batches of the REAL Windows stream
through the chunker. Same code path as the working replay, just less data.

If this works → those Windows bytes are valid, our Python-built batch 2 differs
If this fails → even raw Windows bytes fail in small amounts (would be very weird)

Also: dump our Python-built batch 2 and compare byte-by-byte against Windows batch 2.
"""
import struct
import socket
import time

WIN_BIN = "windows_perfect_stream.bin"

def main():
    with open(WIN_BIN, "rb") as f:
        data = f.read()

    # Parse batch boundaries
    # Batch 0 (META): offset 0, size 20+46=66
    meta_size = struct.unpack('<I', data[16:20])[0]
    batch0_end = 20 + meta_size  # 66
    
    # Batch 1 (JPEG): offset 66
    b1_size = struct.unpack('>I', data[batch0_end+16:batch0_end+20])[0]
    batch1_end = batch0_end + 20 + b1_size
    
    # Batch 2 (JPEG): offset 76619
    b2_size = struct.unpack('>I', data[batch1_end+16:batch1_end+20])[0]
    batch2_end = batch1_end + 20 + b2_size
    
    print(f"=== WINDOWS STREAM BATCH BOUNDARIES ===")
    print(f"Batch 0 (META):  [0, {batch0_end})  = {batch0_end} bytes")
    print(f"Batch 1 (JPEG):  [{batch0_end}, {batch1_end})  = {batch1_end - batch0_end} bytes")
    print(f"Batch 2 (JPEG):  [{batch1_end}, {batch2_end})  = {batch2_end - batch1_end} bytes")
    
    # The raw Windows bytes for first 2 complete frames
    win_first_2 = data[:batch2_end]
    print(f"\nRaw Windows first 2 batches: {len(win_first_2)} bytes")
    
    # === Now build OUR version of 2 batches using Python code ===
    ip_bytes = socket.inet_aton("192.168.88.2")
    
    # Extract tiles from batch 1
    pos = batch0_end + 20 + 4  # skip EPRD header + frame_type
    tiles = []
    for i in range(4):
        x, y, w, h = struct.unpack('>HHHH', data[pos:pos+8])
        flags, ts = struct.unpack('>II', data[pos+8:pos+16])
        pos += 16
        je = data.find(b'\xff\xd9', pos) + 2
        jpeg = data[pos:je]
        tiles.append((x, y, w, h, flags, ts, jpeg))
        pos = je
    
    # Build our batch 1 (with meta) - using EXACT timestamps
    meta = bytes.fromhex("cc0000000400030020200001ff00ff00ff0010080000000006400384000000600400024000000000000000000000")
    meta_hdr = b'EPRD0600' + ip_bytes + struct.pack('<II', 0, len(meta))
    frame_type = struct.pack('>I', 4)
    
    tile_data_1 = bytearray()
    for (x, y, w, h, flags, ts, jpeg) in tiles:
        region = struct.pack('>HHHH', x, y, w, h) + struct.pack('>II', flags, ts)
        tile_data_1 += region + jpeg
    
    jpeg_payload_1 = len(frame_type) + len(tile_data_1)
    jpeg_hdr_1 = b'EPRD0600' + ip_bytes + struct.pack('>II', 0, jpeg_payload_1)
    
    our_batch_1 = meta_hdr + meta + jpeg_hdr_1 + frame_type + bytes(tile_data_1)
    
    # Build our batch 2 (no meta, same tiles, same timestamps) 
    tile_data_2 = bytearray()
    for (x, y, w, h, flags, ts, jpeg) in tiles:
        region = struct.pack('>HHHH', x, y, w, h) + struct.pack('>II', flags, ts)
        tile_data_2 += region + jpeg
    
    jpeg_payload_2 = len(frame_type) + len(tile_data_2)
    jpeg_hdr_2 = b'EPRD0600' + ip_bytes + struct.pack('>II', 0, jpeg_payload_2)
    
    our_batch_2 = jpeg_hdr_2 + frame_type + bytes(tile_data_2)
    
    our_first_2 = our_batch_1 + our_batch_2
    
    print(f"\n=== COMPARISON ===")
    print(f"Windows bytes (2 batches): {len(win_first_2)}")
    print(f"Our Python bytes (2 batches): {len(our_first_2)}")
    print(f"Win batch 1: {len(data[:batch1_end])} bytes")
    print(f"Our batch 1: {len(our_batch_1)} bytes")
    print(f"Batch 1 match: {data[:batch1_end] == our_batch_1}")
    
    # Detailed batch 2 comparison
    win_batch_2_raw = data[batch1_end:batch2_end]
    print(f"\nWin batch 2: {len(win_batch_2_raw)} bytes")
    print(f"Our batch 2: {len(our_batch_2)} bytes")
    
    # Compare first 40 bytes of each batch 2
    print(f"\nWin batch 2 first 40: {win_batch_2_raw[:40].hex()}")
    print(f"Our batch 2 first 40: {our_batch_2[:40].hex()}")
    
    # Win batch 2 has DIFFERENT tiles! Let's see what they are
    b2_ft = struct.unpack('>I', win_batch_2_raw[20:24])[0]
    print(f"\nWin batch 2 frame_type: {b2_ft}")
    b2_pos = 24
    for i in range(4):
        if b2_pos + 16 > len(win_batch_2_raw):
            break
        x, y, w, h = struct.unpack('>HHHH', win_batch_2_raw[b2_pos:b2_pos+8])
        flags, ts = struct.unpack('>II', win_batch_2_raw[b2_pos+8:b2_pos+16])
        print(f"  Win tile {i}: X={x} Y={y} W={w} H={h} flags=0x{flags:08x}")
        b2_pos += 16
        je = win_batch_2_raw.find(b'\xff\xd9', b2_pos) + 2
        if je > 1:
            print(f"    JPEG: {je - b2_pos} bytes")
            b2_pos = je

    print(f"\n=== SENDING RAW WINDOWS FIRST 2 BATCHES ===")
    print(f"(This is a subset of what Option 2 sends and that works)")
    
    # Connect
    from epson_projector.client import EpsonEasyMPClient
    from epson_projector.wifi import interactive_wifi_setup, revert_wifi
    
    print("\n--- Wi-Fi Setup ---")
    connected, orig_uuid, proj_ssid_to_delete = interactive_wifi_setup()
    
    try:
        client = EpsonEasyMPClient()
        success = client.connect_and_negotiate()
        
        if success:
            # Patch IP
            actual_ip_bytes = socket.inet_aton(client.my_ip)
            payload = win_first_2.replace(ip_bytes, actual_ip_bytes)
            
            print(f"\n[*] Sending RAW Windows first 2 batches: {len(payload)} bytes")
            
            chunk_size = 1460
            chunks_sent = 0
            try:
                for i in range(0, len(payload), chunk_size):
                    chunk = payload[i:i+chunk_size]
                    client.s_video.sendall(chunk)
                    time.sleep(0.002)
                    chunks_sent += 1
                
                print(f"[+] SUCCESS! Sent {chunks_sent} chunks. No RST!")
                print(f"[!] Check the wall!")
                
                for i in range(15):
                    client._send_frame_keepalive()
                    time.sleep(1)
                    
            except ConnectionResetError as e:
                print(f"[-] RST after {chunks_sent} chunks ({chunks_sent * 1460} bytes)")
                print(f"    Batch 1 ends at byte ~{batch1_end}")
                print(f"    RST at approximately byte {chunks_sent * 1460}")
                if chunks_sent * 1460 > batch1_end:
                    print(f"    -> RST is DURING batch 2 (offset ~{chunks_sent*1460 - batch1_end} into batch 2)")
                else:
                    print(f"    -> RST is DURING batch 1")
        
        client.disconnect()
        
    except KeyboardInterrupt:
        print("\n[*] Exiting...")
    except Exception as e:
        print(f"[-] Error: {e}")
        import traceback
        traceback.print_exc()
    finally:
        print("\n--- Clean Up ---")
        revert_wifi(orig_uuid, proj_ssid_to_delete)

if __name__ == "__main__":
    main()
