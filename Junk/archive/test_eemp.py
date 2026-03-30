import socket
import struct
import time

PROJECTOR_IP = '192.168.88.1'

def test_payload(payload_name, payload):
    print(f"Testing {payload_name} (len={len(payload)})")
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(2)
        s.connect((PROJECTOR_IP, 3620))
        s.sendall(payload)
        data = s.recv(1024)
        if data:
            print(f"[+] SUCCESS! Response (len={len(data)}): {data.hex()}")
        else:
            print("[-] Connection closed.")
        s.close()
    except Exception as e:
        print(f"[-] Failed: {e}")

pin = b"2270"
padding = b"\x00" * 40

print("Testing Endianness")
test_payload("Big Endian v1, cmd1", struct.pack(">4sHH4s40s", b"EEMP", 1, 1, pin, padding))
test_payload("Big Endian v256, cmd256 (0x0100)", struct.pack(">4sHH4s40s", b"EEMP", 256, 256, pin, padding))
test_payload("Little Endian v1, cmd1", struct.pack("<4sHH4s40s", b"EEMP", 1, 1, pin, padding))
test_payload("Little Endian v2, cmd1", struct.pack("<4sHH4s40s", b"EEMP", 2, 1, pin, padding))
test_payload("Little Endian v1, cmd2", struct.pack("<4sHH4s40s", b"EEMP", 1, 2, pin, padding))

pin_nt = b"2270\x00"
test_payload("PIN Null Term LE v1", struct.pack("<4sHH5s39s", b"EEMP", 1, 1, pin_nt, b"\x00"*39))
