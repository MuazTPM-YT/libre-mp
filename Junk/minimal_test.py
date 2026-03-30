#!/usr/bin/env python3
"""
MINIMAL CHANGE TEST: Take the exact raw Windows stream and change 
exactly ONE byte (the last byte of the first timestamp).
If even this single byte change causes RST, the projector checksums the payload.
"""
import struct
import socket
import time
from epson_projector.client import EpsonEasyMPClient
from epson_projector.wifi import interactive_wifi_setup, revert_wifi

def main():
    with open("windows_perfect_stream.bin", "rb") as f:
        raw = bytearray(f.read())
    
    # First timestamp is at offset 102-105 (verified from binary diff)
    orig_ts = raw[102:106]
    print(f"Original timestamp at offset 102: {orig_ts.hex()}")
    
    print("\n--- Wi-Fi Setup ---")
    connected, orig_uuid, proj_ssid_to_delete = interactive_wifi_setup()
    
    try:
        client = EpsonEasyMPClient()
        success = client.connect_and_negotiate()
        if not success: return
        
        old_ip = socket.inet_aton("192.168.88.2")
        new_ip = socket.inet_aton(client.my_ip)
        
        print(f"\nclient.my_ip = {client.my_ip}")
        print(f"old_ip bytes = {old_ip.hex()}")
        print(f"new_ip bytes = {new_ip.hex()}")
        print(f"IPs match: {old_ip == new_ip}")
        
        print(f"\n[1] Raw Windows UNMODIFIED (control)")
        print(f"[2] Flip ONE bit in first timestamp (byte 105: 02 → 03)")
        print(f"[3] Flip ONE bit in LAST byte of stream (zero-risk position)")
        choice = input("(1-3): ").strip()
        
        stream = bytearray(raw)
        
        if choice == "2":
            stream[105] = 0x03  # Change just the last byte of first timestamp
            print(f"Patched byte 105: {raw[105]:02x} → {stream[105]:02x}")
        elif choice == "3":
            stream[-1] = (stream[-1] + 1) & 0xFF
            print(f"Patched last byte: {raw[-1]:02x} → {stream[-1]:02x}")
        
        # Apply IP replacement (should be no-op)
        data = bytes(stream)
        if old_ip != new_ip:
            print(f"WARNING: IPs differ! replace() will modify {data.count(old_ip)} occurrences!")
            data = data.replace(old_ip, new_ip)
        else:
            print("IPs match - no replacement needed")
        
        print(f"\nSending {len(data)} bytes...")
        chunks = 0
        try:
            for i in range(0, len(data), 1460):
                client.s_video.sendall(data[i:i+1460])
                time.sleep(0.002)
                chunks += 1
            print(f"[+] {chunks} chunks sent, no RST!")
            print("[!] CHECK THE WALL!")
            while True:
                time.sleep(1)
        except (ConnectionResetError, BrokenPipeError) as e:
            print(f"[-] {e} at chunk {chunks}")
        
        client.disconnect()
    except KeyboardInterrupt:
        print("\n[*] Bye")
    except Exception as e:
        import traceback; traceback.print_exc()
    finally:
        print("\n--- Clean Up ---")
        revert_wifi(orig_uuid, proj_ssid_to_delete)

if __name__ == "__main__":
    main()
