import sys
import time

try:
    from PIL import Image
    import numpy as np
except ImportError:
    import os
    os.system('pip install Pillow numpy')
    from PIL import Image
    import numpy as np

from epson_projector.client import EpsonEasyMPClient

def test():
    print("Initializing client...")
    client = EpsonEasyMPClient()
    print("Connecting...")
    success = client.connect_and_negotiate()
    if not success:
        print("Negotiation failed.")
        return
    
    print("Negotiation successful! Sending a test frame...")
    img = Image.new('RGB', (624, 416), color = 'red')
    
    # Save to memory as JPEG
    import io
    mem_file = io.BytesIO()
    img.save(mem_file, 'JPEG', quality=80)
    jpeg_bytes = mem_file.getvalue()
    
    client.send_video_frame(0, 0, 624, 416, jpeg_bytes)
    print("Frame 1 sent. Sleeping 2s...")
    time.sleep(2)
    
    img = Image.new('RGB', (624, 416), color = 'green')
    mem_file = io.BytesIO()
    img.save(mem_file, 'JPEG', quality=80)
    jpeg_bytes = mem_file.getvalue()
    
    client.send_video_frame(0, 0, 624, 416, jpeg_bytes)
    print("Frame 2 sent. Sleeping 2s...")
    time.sleep(2)
    
    client.disconnect()
    print("Done!")

if __name__ == '__main__':
    test()
