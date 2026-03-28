#!/usr/bin/env python3
"""
REPLAY TEST: Send python_built_stream.bin through the EXACT same 
code path as main.py's working Windows replay.

If this WORKS → our bytes are fine, there's a TCP behavior diff in sustained_test
If this FAILS → our bytes are wrong (but we can't find the difference)
"""
import socket
import time
from epson_projector.client import EpsonEasyMPClient
from epson_projector.wifi import interactive_wifi_setup, revert_wifi

def replay(client, filepath):
    with open(filepath, "rb") as f:
        payload = f.read()
    
    old_ip = socket.inet_aton("192.168.88.2")
    new_ip = socket.inet_aton(client.my_ip)
    payload = payload.replace(old_ip, new_ip)
    
    print(f"[*] Replaying {filepath}: {len(payload)} bytes")
    chunk_size = 1460
    for i in range(0, len(payload), chunk_size):
        chunk = payload[i:i+chunk_size]
        client.s_video.sendall(chunk)
        time.sleep(0.002)
    print(f"[+] Replay finished!")
    
    # Hold connection (same as main.py)
    while True:
        time.sleep(1)

def main():
    print("--- Wi-Fi Setup ---")
    connected, orig_uuid, proj_ssid_to_delete = interactive_wifi_setup()
    
    try:
        client = EpsonEasyMPClient()
        success = client.connect_and_negotiate()
        
        if success:
            print("\nWhich file to replay?")
            print("[1] windows_perfect_stream.bin (control)")
            print("[2] python_built_stream.bin (test)")
            choice = input("(1 or 2): ").strip()
            
            if choice == "2":
                replay(client, "python_built_stream.bin")
            else:
                replay(client, "windows_perfect_stream.bin")
        
        client.disconnect()
    except KeyboardInterrupt:
        print("\n[*] Exiting...")
    except Exception as e:
        print(f"[-] Error: {e}")
        import traceback; traceback.print_exc()
    finally:
        print("\n--- Clean Up ---")
        revert_wifi(orig_uuid, proj_ssid_to_delete)

if __name__ == "__main__":
    main()
