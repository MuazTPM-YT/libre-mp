import socket

# --- Configuration ---
# Update this to your Projector's actual IP address on the network
PROJECTOR_IP = '169.254.160.040'

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
        return '127.0.0.1'

# Automatically grab the local IP of this machine, or hardcode it
MY_IP = get_local_ip()  # Previously '192.168.88.3'

PORT_CONTROL = 3620
PORT_VIDEO = 3621
PORT_WAKE = 3629
