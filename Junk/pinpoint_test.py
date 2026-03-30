#!/usr/bin/env python3
"""
TWO PINPOINT TESTS:

Test 1: Windows batch 0+1 (raw) → Python batch 2 with SAME tiles as batch 1
  If RST → projector rejects identical consecutive tiles
  If OK  → something else is wrong in continuous_test

Test 2: Python batch 0+1 (with time.time() timestamps) → Python batch 2 with batch 2's tiles
  If RST → our Python-built batch 1 is rejected (timestamps or other issue)
  If OK  → Python batch 1 is fine; continuous_test failure was from tile reuse
"""
import struct
import socket
import time

WIN_BIN = "windows_perfect_stream.bin"

def parse_batches(data, count=5):
    batches = []
    pos = 0
    while pos < len(data) and len(batches) < count:
        if data[pos:pos+8] != b'EPRD0600': break
        size_le = struct.unpack('<I', data[pos+16:pos+20])[0]
        size_be = struct.unpack('>I', data[pos+16:pos+20])[0]
        fb = data[pos+20]
        ps = size_le if fb == 0xCC else size_be
        batches.append(data[pos:pos+20+ps])
        pos += 20 + ps
    return batches

def extract_tiles(batch_raw):
    payload = batch_raw[20:]
    pos = 4  # skip frame_type
    tiles = []
    while pos + 16 <= len(payload):
        x, y, w, h = struct.unpack('>HHHH', payload[pos:pos+8])
        if w == 0 or h == 0 or w > 2000: break
        pos += 16
        if payload[pos:pos+2] != b'\xff\xd8': break
        je = payload.find(b'\xff\xd9', pos) + 2
        tiles.append((x, y, w, h, payload[pos:je]))
        pos = je
    return tiles

def build_batch(ip, frame_type_val, tiles):
    ft = struct.pack('>I', frame_type_val)
    td = bytearray()
    for (x, y, w, h, jpeg) in tiles:
        ts = int(time.time() * 1000) & 0xFFFFFFFF
        td += struct.pack('>HHHH', x, y, w, h) + struct.pack('>II', 7, ts) + jpeg
    ps = len(ft) + len(td)
    hdr = b'EPRD0600' + ip + struct.pack('>II', 0, ps)
    return hdr + ft + bytes(td)

def build_meta_block(ip):
    meta = bytes.fromhex("cc0000000400030020200001ff00ff00ff0010080000000006400384000000600400024000000000000000000000")
    meta_hdr = b'EPRD0600' + ip + struct.pack('<II', 0, len(meta))
    return meta_hdr + meta

def send_stream(sock, buf):
    sent = 0
    for i in range(0, len(buf), 1460):
        chunk = buf[i:i+1460]
        sock.sendall(chunk)
        time.sleep(0.002)
        sent += 1
    return sent

def main():
    with open(WIN_BIN, "rb") as f:
        data = f.read()
    
    ip = socket.inet_aton("192.168.88.2")
    batches = parse_batches(data, 3)
    tiles_b1 = extract_tiles(batches[1])
    tiles_b2 = extract_tiles(batches[2])
    
    print("=== Tiles from batch 1 ===")
    for i, (x,y,w,h,j) in enumerate(tiles_b1):
        print(f"  {i}: ({x},{y}) {w}x{h} JPEG={len(j)}B")
    print("=== Tiles from batch 2 ===")
    for i, (x,y,w,h,j) in enumerate(tiles_b2):
        print(f"  {i}: ({x},{y}) {w}x{h} JPEG={len(j)}B")
    
    from epson_projector.client import EpsonEasyMPClient
    from epson_projector.wifi import interactive_wifi_setup, revert_wifi
    
    print("\n--- Wi-Fi Setup ---")
    connected, orig_uuid, proj_ssid_to_delete = interactive_wifi_setup()
    
    try:
        client = EpsonEasyMPClient()
        success = client.connect_and_negotiate()
        if not success:
            return
        
        actual_ip = socket.inet_aton(client.my_ip)
        
        print("\nWhich test to run?")
        print("[1] Win batch 0+1 raw → Python batch 2 with SAME tiles as batch 1")
        print("[2] Python batch 0+1 → Python batch 2 with batch 2's tiles")
        choice = input("Select (1 or 2): ").strip()
        
        if choice == "1":
            print(f"\n{'='*60}")
            print("TEST 1: Windows raw batch 0+1 → Python batch 2 with BATCH 1 tiles")
            print(f"{'='*60}")
            
            raw_b01 = batches[0] + batches[1]
            raw_b01 = raw_b01.replace(ip, actual_ip) if ip != actual_ip else raw_b01
            py_b2 = build_batch(actual_ip, 4, tiles_b1)  # SAME tiles as batch 1!
            
            stream = raw_b01 + py_b2
            print(f"  Total: {len(stream)} bytes")
            
            try:
                n = send_stream(client.s_video, stream)
                print(f"  [+] SUCCESS! {n} chunks, no RST!")
                print(f"  → Tile reuse is NOT the issue!")
                for i in range(10):
                    client._send_frame_keepalive()
                    time.sleep(1)
            except ConnectionResetError:
                print(f"  [-] RST! Projector rejects identical consecutive tiles!")
        
        elif choice == "2":
            print(f"\n{'='*60}")
            print("TEST 2: Python batch 0+1 (time.time) → Python batch 2 (batch 2 tiles)")
            print(f"{'='*60}")
            
            py_meta = build_meta_block(actual_ip)
            py_b1 = build_batch(actual_ip, 4, tiles_b1)
            py_b2 = build_batch(actual_ip, 4, tiles_b2)
            
            stream = py_meta + py_b1 + py_b2
            print(f"  Total: {len(stream)} bytes")
            
            try:
                n = send_stream(client.s_video, stream)
                print(f"  [+] SUCCESS! {n} chunks, no RST!")
                print(f"  → Python batch 1 is fine! Issue was tile reuse.")
                for i in range(10):
                    client._send_frame_keepalive()
                    time.sleep(1)
            except ConnectionResetError:
                print(f"  [-] RST! Our Python-built batch 1 is subtly broken!")
                print(f"  → Issue is in batch 1 headers, NOT tile reuse.")
        
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
