import time
import socket
from epson_projector.client import EpsonEasyMPClient

def main():
    print("--- Pure Windows Replay (Dynamic IP Patch) ---")
    
    # 1. Connect and handshakes
    client = EpsonEasyMPClient()
    success = client.connect_and_negotiate()
    if not success:
        print("[-] Handshake failed. Cannot replay.")
        return
        
    print(f"[+] Handshake successful. Our IP is: {client.my_ip}")
    print("[*] Replaying 'windows_perfect_stream.bin'...")
    
    try:
        with open("windows_perfect_stream.bin", "rb") as f:
            windows_payload = f.read()
            
        # The windows_perfect_stream.bin has the Windows IP hardcoded as 192.168.88.2 (c0 a8 58 02)
        # We MUST patch the EPRD headers to use our Arch Linux interface IP, or the projector will reject it!
        old_ip_bytes = socket.inet_aton("192.168.88.2")
        new_ip_bytes = socket.inet_aton(client.my_ip)
        
        # Safe replacement: only replace when it follows the 'EPRD0600' signature
        windows_payload = windows_payload.replace(
            b'EPRD0600' + old_ip_bytes, 
            b'EPRD0600' + new_ip_bytes
        )
            
        print(f"[*] Payload dynamically patched for {client.my_ip}. Length: {len(windows_payload)} bytes.")
        print("[*] Transmitting in TCP chunks...")
        
        # We send it in 1460-byte chunks to perfectly mimic the TCP MTU segmentation
        chunk_size = 1460
        for i in range(0, len(windows_payload), chunk_size):
            chunk = windows_payload[i:i+chunk_size]
            client.s_video.sendall(chunk)
            # A tiny sleep forces the OS to send these as separate PSH packets!
            time.sleep(0.002)
            
        print("[+] Replay finished successfully! Did the Windows desktop appear on the wall?")
        
        # Keep alive
        print("[*] Sending keepalives. Press Ctrl+C to close.")
        while True:
            client._send_frame_keepalive()
            time.sleep(1.0)
            
    except KeyboardInterrupt:
        pass
    except Exception as e:
        print(f"[-] Replay interrupted: {e}")
    finally:
        print("[*] Disconnecting...")
        client.disconnect()

if __name__ == "__main__":
    main()
