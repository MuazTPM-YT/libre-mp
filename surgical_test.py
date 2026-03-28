#!/usr/bin/env python3
"""
SURGICAL ISOLATION TEST

Test A: Windows batch 0+1 (raw) + Python-built batch 2 using Windows batch 2's tiles
  → If works: our EPRD header building is correct; continuous_test failed because of tile reuse
  → If fails: our EPRD header building is subtly wrong

Test B: Windows batch 0+1 (raw) + raw Windows batch 2 but with EPRD header rebuilt by Python
  → If works: Python EPRD header is correct
  → If fails: our EPRD header bytes differ from Windows

Test C: 3 copies of the raw Windows batch 0+1+2 concatenated
  → Tests if the projector handles multiple batches with same tile data
"""
import struct
import socket
import time

WIN_BIN = "windows_perfect_stream.bin"

def parse_windows_batches(data):
    """Parse the first 3 EPRD blocks from the stream."""
    batches = []
    pos = 0
    while pos < len(data) and len(batches) < 5:
        if data[pos:pos+8] != b'EPRD0600':
            break
        size_le = struct.unpack('<I', data[pos+16:pos+20])[0]
        size_be = struct.unpack('>I', data[pos+16:pos+20])[0]
        first_byte = data[pos+20]
        
        if first_byte == 0xCC:
            payload_size = size_le
            btype = 'META'
        else:
            payload_size = size_be
            btype = 'JPEG'
        
        block = data[pos:pos+20+payload_size]
        batches.append({
            'type': btype,
            'offset': pos,
            'size': 20 + payload_size,
            'payload_size': payload_size,
            'raw': block,
            'header': data[pos:pos+20],
            'payload': data[pos+20:pos+20+payload_size],
        })
        pos += 20 + payload_size
    return batches

def extract_tiles_from_batch(batch_payload):
    """Extract tiles from a JPEG batch payload (after EPRD header)."""
    ft = struct.unpack('>I', batch_payload[:4])[0]
    pos = 4
    tiles = []
    while pos + 16 <= len(batch_payload):
        x, y, w, h = struct.unpack('>HHHH', batch_payload[pos:pos+8])
        flags, ts = struct.unpack('>II', batch_payload[pos+8:pos+16])
        if w == 0 or h == 0 or w > 2000 or h > 2000:
            break
        pos += 16
        if batch_payload[pos:pos+2] != b'\xff\xd8':
            break
        je = batch_payload.find(b'\xff\xd9', pos) + 2
        jpeg = batch_payload[pos:je]
        tiles.append((x, y, w, h, jpeg))
        pos = je
    return ft, tiles

def build_jpeg_batch(ip_bytes, frame_type_val, tiles):
    """Build a complete EPRD JPEG batch from scratch."""
    frame_type = struct.pack('>I', frame_type_val)
    tile_data = bytearray()
    for (x, y, w, h, jpeg) in tiles:
        ts = int(time.time() * 1000) & 0xFFFFFFFF
        region = struct.pack('>HHHH', x, y, w, h) + struct.pack('>II', 0x00000007, ts)
        tile_data += region + jpeg
    
    payload_size = len(frame_type) + len(tile_data)
    header = b'EPRD0600' + ip_bytes + struct.pack('>II', 0, payload_size)
    return header + frame_type + bytes(tile_data)

def main():
    with open(WIN_BIN, "rb") as f:
        data = f.read()
    
    ip_bytes = socket.inet_aton("192.168.88.2")
    
    batches = parse_windows_batches(data)
    print("=== PARSED WINDOWS BATCHES ===")
    for i, b in enumerate(batches):
        print(f"  Block {i}: {b['type']} offset={b['offset']} size={b['size']}")
    
    # Extract tiles from batch 2 (second JPEG block)
    jpeg_batch_2 = batches[2]  # Third block overall = second JPEG batch
    ft2, tiles2 = extract_tiles_from_batch(jpeg_batch_2['payload'])
    print(f"\n  Batch 2 frame_type: {ft2}, tiles: {len(tiles2)}")
    for i, (x, y, w, h, jpeg) in enumerate(tiles2):
        print(f"    Tile {i}: ({x},{y}) {w}x{h} JPEG={len(jpeg)}B")
    
    # Build our Python version of batch 2 using Windows batch 2's tiles
    our_batch2 = build_jpeg_batch(ip_bytes, ft2, tiles2)
    
    # Compare headers
    print(f"\n=== BATCH 2 HEADER COMPARISON ===")
    print(f"  Windows: {jpeg_batch_2['header'].hex()}")
    print(f"  Ours:    {our_batch2[:20].hex()}")
    # The payload sizes should match since we use the same tiles:
    win_payload_size = jpeg_batch_2['payload_size']
    our_payload_size = struct.unpack('>I', our_batch2[16:20])[0]
    print(f"  Win payload size: {win_payload_size}")
    print(f"  Our payload size: {our_payload_size}")
    
    # The first 24 bytes after header should match (frame_type + first region)
    print(f"  Win after header: {jpeg_batch_2['payload'][:24].hex()}")
    print(f"  Our after header: {our_batch2[20:44].hex()}")
    
    # Connect
    from epson_projector.client import EpsonEasyMPClient
    from epson_projector.wifi import interactive_wifi_setup, revert_wifi
    
    print("\n--- Wi-Fi Setup ---")
    connected, orig_uuid, proj_ssid_to_delete = interactive_wifi_setup()
    
    try:
        client = EpsonEasyMPClient()
        success = client.connect_and_negotiate()
        
        if not success:
            print("[-] Connection failed")
            return
        
        actual_ip = socket.inet_aton(client.my_ip)
        
        # === TEST A: Windows batch 0+1 + Python-built batch 2 ===
        print(f"\n{'='*60}")
        print("TEST A: Windows batch 0+1 + PYTHON-BUILT batch 2")
        print(f"{'='*60}")
        
        win_b01 = batches[0]['raw'] + batches[1]['raw']
        py_b2 = build_jpeg_batch(actual_ip, ft2, tiles2)
        
        test_a_stream = win_b01.replace(ip_bytes, actual_ip) + py_b2
        
        print(f"  Stream: {len(test_a_stream)} bytes")
        
        chunk_size = 1460
        chunks_sent = 0
        try:
            for i in range(0, len(test_a_stream), chunk_size):
                chunk = test_a_stream[i:i+chunk_size]
                client.s_video.sendall(chunk)
                time.sleep(0.002)
                chunks_sent += 1
            
            print(f"  [+] TEST A SUCCESS! {chunks_sent} chunks sent!")
            print(f"  [!] Check the wall!")
            
            for i in range(10):
                client._send_frame_keepalive()
                time.sleep(1)
                
        except ConnectionResetError:
            batch1_chunks = (batches[0]['size'] + batches[1]['size']) // 1460 + 1
            print(f"  [-] TEST A FAILED at chunk {chunks_sent}")
            print(f"      Batch 0+1 ends at ~chunk {batch1_chunks}")
            if chunks_sent > batch1_chunks:
                print(f"      -> RST during PYTHON batch 2 (our header is wrong!)")
            else:
                print(f"      -> RST during Windows batch 1 (projector rejected it)")
        
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
