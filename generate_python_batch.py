import io
import struct
from PIL import Image
from epson_projector.client import EpsonEasyMPClient

class MockSocket:
    def __init__(self):
        self.data = bytearray()
    def sendall(self, b):
        self.data.extend(b)

client = EpsonEasyMPClient()
client.my_ip = '192.168.88.2'
client.s_video = MockSocket()
client.first_frame = True 

frame = Image.new("RGB", (1024, 768), color=(40, 40, 100))

tiles = []
quads = [(0, 0, 624, 416), (624, 0, 400, 416), (0, 416, 624, 352), (624, 416, 400, 352)]
for (x, y, w, h) in quads:
    crop_box = (x, y, x + w, y + h)
    tile_img = frame.crop(crop_box)
    buf = io.BytesIO()
    tile_img.save(buf, format="JPEG", quality=60, subsampling=2, optimize=False, progressive=False)
    tiles.append((x, y, w, h, buf.getvalue()))

client.send_video_batch(tiles)

with open("python_perfect_stream.bin", "wb") as f:
    f.write(client.s_video.data)
print(f"Dumped {len(client.s_video.data)} bytes to python_perfect_stream.bin")
