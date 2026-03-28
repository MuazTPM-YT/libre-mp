#!/usr/bin/env python3
"""
DEFINITIVE TEST: Take the EXACT raw Windows bytes and patch ONLY 
the 4-byte timestamp fields with our time.time() values.
Everything else stays as raw Windows bytes.

If works → timestamps don't matter, previous failures were from something else
If fails → timestamps are critical and must match a specific format
"""
import struct
import socket
import time

WIN_BIN = "windows_perfect_stream.bin"

def main():
    with open(WIN_BIN, "rb") as f:
        data = f.read()
    
    # Parse all batches to find timestamp field offsets
    ip = socket.inet_aton("192.168.88.2")
    
    meta_end = 66
    b1_size = struct.unpack('>I', data[meta_end+16:meta_end+20])[0]
    b1_end = meta_end + 20 + b1_size
    b2_size = struct.unpack('>I', data[b1_end+16:b1_end+20])[0]
    b2_end = b1_end + 20 + b2_size
    
    # Use raw Windows bytes for batches 0+1+2
    raw = bytearray(data[:b2_end])
    
    print(f"Raw Windows first 3 blocks: {len(raw)} bytes")
    
    # Find all timestamp offsets (byte 12-16 of each region descriptor)
    # Region: x(2) + y(2) + w(2) + h(2) + flags(4) + ts(4)
    # ts is at offset +12 within each region
    
    # Batch 1 tiles start at offset 90 (66 + 20 + 4)
    ts_offsets = []
    pos = meta_end + 20 + 4  # 90 = after JPEG EPRD header + frame_type
    for i in range(4):
        ts_off = pos + 12
        ts_offsets.append(ts_off)
        x, y, w, h = struct.unpack('>HHHH', raw[pos:pos+8])
        pos += 16
        je = raw.find(b'\xff\xd9', pos) + 2
        pos = je
    
    # Batch 2 tiles
    pos = b1_end + 20 + 4
    for i in range(4):
        ts_off = pos + 12
        ts_offsets.append(ts_off)
        x, y, w, h = struct.unpack('>HHHH', raw[pos:pos+8])
        pos += 16
        je = raw.find(b'\xff\xd9', pos) + 2 
        pos = je
    
    print(f"Found {len(ts_offsets)} timestamp field offsets")
    
    # Show original timestamps
    for i, off in enumerate(ts_offsets):
        orig = struct.unpack('>I', raw[off:off+4])[0]
        print(f"  TS {i}: offset={off} value=0x{orig:08x}")
    
    from epson_projector.client import EpsonEasyMPClient
    from epson_projector.wifi import interactive_wifi_setup, revert_wifi
    
    print("\n--- Wi-Fi Setup ---")
    connected, orig_uuid, proj_ssid_to_delete = interactive_wifi_setup()
    
    try:
        client = EpsonEasyMPClient()
        success = client.connect_and_negotiate()
        if not success: return
        actual_ip = socket.inet_aton(client.my_ip)
        
        print(f"\nWhich test?")
        print("[1] Raw Windows bytes UNMODIFIED (control test - should work)")
        print("[2] Raw Windows bytes with ALL timestamps patched to time.time()")
        print("[3] Raw Windows bytes with ONLY batch 1 timestamps patched")
        print("[4] Raw Windows bytes with ONLY batch 2 timestamps patched")
        choice = input("(1-4): ").strip()
        
        stream = bytearray(raw)
        if actual_ip != ip:
            # Carefully patch only EPRD header IPs, not JPEG data
            # Meta header IP at offset 8
            stream[8:12] = actual_ip
            # Batch 1 JPEG header IP at offset 66+8=74
            stream[74:78] = actual_ip
            # Batch 2 JPEG header IP at offset b1_end+8
            stream[b1_end+8:b1_end+12] = actual_ip
        
        if choice in ("2", "3"):
            # Patch batch 1 timestamps
            for off in ts_offsets[:4]:
                ts = int(time.time() * 1000) & 0xFFFFFFFF
                struct.pack_into('>I', stream, off, ts)
                print(f"  Patched TS at offset {off} → 0x{ts:08x}")
        
        if choice in ("2", "4"):
            # Patch batch 2 timestamps
            for off in ts_offsets[4:]:
                ts = int(time.time() * 1000) & 0xFFFFFFFF
                struct.pack_into('>I', stream, off, ts)
                print(f"  Patched TS at offset {off} → 0x{ts:08x}")
        
        label = {
            "1": "UNMODIFIED (control)",
            "2": "ALL timestamps patched",
            "3": "ONLY batch 1 timestamps patched",
            "4": "ONLY batch 2 timestamps patched"
        }[choice]
        
        print(f"\nSending: {label} ({len(stream)} bytes)")
        
        chunks_sent = 0
        try:
            for i in range(0, len(stream), 1460):
                client.s_video.sendall(bytes(stream[i:i+1460]))
                time.sleep(0.002)
                chunks_sent += 1
            
            print(f"[+] {chunks_sent} chunks sent, no RST!")
            print(f"[!] CHECK THE WALL — is there an image?")
            for i in range(15):
                client._send_frame_keepalive()
                time.sleep(1)
        except ConnectionResetError:
            print(f"[-] RST at chunk {chunks_sent}")
        
        client.disconnect()
    except KeyboardInterrupt: pass
    except Exception as e:
        import traceback; traceback.print_exc()
    finally:
        print("\n--- Clean Up ---")
        revert_wifi(orig_uuid, proj_ssid_to_delete)

if __name__ == "__main__":
    main()
