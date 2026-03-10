from epson_projector.client import EpsonEasyMPClient
from epson_projector.wifi import interactive_wifi_setup, revert_wifi

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
            print("Ready to stream video frames payload over client.s_video!")
            # Example of where you would drop your payload:
            # client.send_video_frame(my_encrypted_video_data)
            
            # For demonstration, keep it alive so we don't instantly drop
            import time
            while True:
                time.sleep(1)
                
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
