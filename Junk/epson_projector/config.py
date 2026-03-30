import socket
import subprocess

# --- Configuration ---
# Epson Projectors default to 192.168.88.1 when hosting their own Wi-Fi or Ad-Hoc networks.
PROJECTOR_IP = '192.168.88.1'

def get_local_ip():
    # Attempt to establish a dummy connection to get the routable local IP address
    # We must use the projector's IP because this network has NO INTERNET
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect((PROJECTOR_IP, 80))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except Exception:
        return '192.168.88.2'  # Standard second IP in the Ad-Hoc block

# Automatically grab the local IP of this machine, or hardcode it
MY_IP = get_local_ip()

PORT_CONTROL = 3620
PORT_VIDEO = 3621
PORT_WAKE = 3629

# --- Display / Streaming ---
# The projector's native display resolution (from pcap display config meta)
PROJECTOR_DISPLAY_WIDTH = 1600
PROJECTOR_DISPLAY_HEIGHT = 900

# The full VNC viewport resolution the projector expects.
# Windows slices 1600x900 into a 1024x768 grid (matching VNC conventions).
STREAM_WIDTH = 1024
STREAM_HEIGHT = 768

# The exact 4-tile grid from the Windows PCAP binary analysis.
# Every frame is sliced into these regions; the projector hardware expects
# all 4 tiles packed into a single EPRD JPEG batch.
# Each tuple: (x_offset, y_offset, width, height)
TILE_GRID = [
    (0,   0,   624, 416),
    (624, 0,   400, 416),
    (0,   416, 624, 352),
    (624, 416, 400, 352),
]

# JPEG compression quality for streamed frames (pcap shows ~50 quality)
JPEG_QUALITY = 50
