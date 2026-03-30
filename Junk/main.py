from epson_projector.client import EpsonEasyMPClient
from epson_projector.wifi import interactive_wifi_setup, revert_wifi
from epson_projector.video_streamer import VideoStreamer

def main():
    print("--- Wi-Fi Setup ---")
    connected, orig_uuid, proj_ssid_to_delete = interactive_wifi_setup()
    
    if not connected:
        print("Warning: Network setup was skipped or failed. The script will try to continue anyway.")

    try:
        print("\n--- Initializing Projector Client ---")
        client = EpsonEasyMPClient()
        
        success = client.connect_and_negotiate()
        
        if success:
            print("\\n--- Stream Mode ---")
            print("[1] Live Wayland capture (grim)")
            print("[2] Pure Windows PCAP Replay (windows_perfect_stream.bin)")
            choice = input("Select mode (1 or 2, default=1): ").strip()
            
            if choice == "2":
                print("[*] Replaying 'windows_perfect_stream.bin'...")
                import socket, time
                try:
                    with open("windows_perfect_stream.bin", "rb") as f:
                        windows_payload = f.read()
                    
                    # Patch IP dynamically
                    old_ip_bytes = socket.inet_aton("192.168.88.2")
                    new_ip_bytes = socket.inet_aton(client.my_ip)
                    windows_payload = windows_payload.replace(old_ip_bytes, new_ip_bytes)
                    
                    print(f"[*] Payload dynamically patched for {client.my_ip}. Transmitting in TCP chunks...")
                    chunk_size = 1460
                    for i in range(0, len(windows_payload), chunk_size):
                        chunk = windows_payload[i:i+chunk_size]
                        client.s_video.sendall(chunk)
                        time.sleep(0.002)
                        
                    print("[+] Replay finished successfully! Is the static Windows image displayed on the wall?")
                    while True:
                        time.sleep(1)
                except Exception as e:
                    print(f"[-] Replay failed: {e}")
            else:
                print("[+] Wayland detected. Using 'grim' for screen capture.")
                streamer = VideoStreamer(client, fps=24)
                streamer.start_streaming()
            
        client.disconnect()
        
    except KeyboardInterrupt:
        print("\n[*] Exiting script...")
        try:
            client.disconnect()
        except:
            pass
    finally:
        print("\n--- Clean Up ---")
        revert_wifi(orig_uuid, proj_ssid_to_delete)

if __name__ == "__main__":
    main()

