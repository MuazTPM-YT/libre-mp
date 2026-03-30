import socket
import struct

PROJECTOR_IP = '192.168.88.1'

def test_eemp(pin_str="2270"):
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.settimeout(5)
    s.connect((PROJECTOR_IP, 3620))
    print("[*] Connected to port 3620")

    # STEP 1: Read what the projector sends first
    banner = s.recv(1024)
    print(f"[*] Projector banner ({len(banner)} bytes):\n    {banner.hex()}")
    print(f"[*] ASCII attempt: {banner}")

    # STEP 2: Parse the banner to understand the challenge
    # Look at your pcap to confirm structure, but typically:
    # Bytes 0-3:  "EEMP"
    # Bytes 4-5:  version (little or big endian)
    # Bytes 6-7:  command/state
    # Remaining:  challenge data / random bytes
    if banner[:4] == b'EEMP':
        print("[+] Got EEMP banner!")
        version = struct.unpack_from("<H", banner, 4)[0]
        command = struct.unpack_from("<H", banner, 6)[0]
        print(f"    Version (LE): {version}, Command (LE): {command}")
        version_be = struct.unpack_from(">H", banner, 4)[0]
        command_be = struct.unpack_from(">H", banner, 6)[0]
        print(f"    Version (BE): {version_be}, Command (BE): {command_be}")
    else:
        print(f"[-] Not EEMP? Raw: {banner[:20].hex()}")
        s.close()
        return

    # STEP 3: Build auth response
    # Try matching the same version/endianness as projector's banner
    pin_bytes = pin_str.encode().ljust(4, b'\x00')  # "2270" padded
    padding = b'\x00' * 40

    # Try little endian response matching projector's version
    payload = struct.pack("<4sHH4s40s",
        b"EEMP",
        version,    # echo back same version
        2,          # command 2 = auth request (common pattern)
        pin_bytes,
        padding
    )

    print(f"\n[*] Sending auth payload ({len(payload)} bytes):\n    {payload.hex()}")
    s.sendall(payload)

    # STEP 4: Read the result
    response = s.recv(1024)
    print(f"\n[*] Auth response ({len(response)} bytes):\n    {response.hex()}")

    # Check the 51st byte (index 50)
    if len(response) > 50:
        auth_byte = response[50]
        print(f"\n[*] Key byte [50] = 0x{auth_byte:02x}")
        if auth_byte != 0:
            print("[+] ✅ AUTH SUCCESS!")
        else:
            print("[-] ❌ Auth failed (byte 50 is 0x00)")
    else:
        print(f"[!] Response too short ({len(response)} bytes) to check byte 50")
        print(f"    Full response: {response}")

    s.close()

# Test with backdoor PIN first
test_eemp("2270")