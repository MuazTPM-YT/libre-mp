import socket

PROJECTOR_IP = '192.168.88.1'
MY_IP = '192.168.88.3'

# 45454d50 30313030 = EEMP0100
hex_ip_normal = socket.inet_aton(MY_IP).hex()
PAYLOAD_CONTROL = bytes.fromhex(f"45454d5030313030{hex_ip_normal}1001000000000000")

s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.settimeout(5)
s.connect((PROJECTOR_IP, 3620))
s.send(PAYLOAD_CONTROL)
print("Sent EEMP0100 to Control Port 3620, waiting for reply...")
try:
    resp = s.recv(1024)
    if resp:
        print(f"Reply: {resp.hex()}")
    else:
        print("Empty reply")
except Exception as e:
    print(f"Failed: {e}")
finally:
    s.close()
