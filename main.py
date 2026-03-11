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

