import sys
sys.path.append('.')
from epson_projector.client import EpsonEasyMPClient

try:
    print("[*] Starting fast debug run...")
    client = EpsonEasyMPClient()
    success = client.connect_and_negotiate()
    if success:
        print("[+] Connecting successful! Sending test frame...")
        # send a tiny test frame
        jpeg_bytes = b'\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00\xff\xdb\x00C\x00' + (b'\x00'*100)
        client.send_video_frame(0, 0, 624, 416, jpeg_bytes)
    client.disconnect()
except Exception as e:
    print(f"Error: {e}")
