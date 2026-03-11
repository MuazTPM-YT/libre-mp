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
        return '192.168.88.2' # Standard second IP in the Ad-Hoc block

# Automatically grab the local IP of this machine, or hardcode it
MY_IP = get_local_ip()

PORT_CONTROL = 3620
PORT_VIDEO = 3621
PORT_WAKE = 3629

