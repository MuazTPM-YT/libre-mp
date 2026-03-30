import socket
import time

PROJECTOR_IP = '192.168.88.1'

print(f"Testing basic TCP connectivity to {PROJECTOR_IP}...")

def check_port(port):
    print(f"Checking Port {port}...")
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(2)
        s.connect((PROJECTOR_IP, port))
        print(f"  [+] Port {port} is OPEN and accepted TCP connection.")
        s.close()
    except Exception as e:
        print(f"  [-] Port {port} failed: {e}")

check_port(3620)
check_port(3621)
check_port(3629)
check_port(80)
