import socket
import struct
import binascii

def get_hex_ip(ip: str) -> str:
    return socket.inet_aton(ip).hex()

def get_hex_ip_reversed(ip: str) -> str:
    return socket.inet_aton(ip)[::-1].hex()

def get_registration_payload(my_ip: str) -> bytes:
    """
    Returns the exact 68-byte first packet sent by the official Epson client
    on the first TCP connection (Port 3620) to register the client IP.
    """
    hex_ip = get_hex_ip(my_ip)
    p = (
        f"45454d5030313030{hex_ip}0200000030000000007f0000b0f8ef5314000000"
        f"0000000000000000000000000000000000000000000000000000000000000000"
    )
    return binascii.unhexlify(p)

def get_auth_payload_full(my_ip: str, proj_ip: str, my_mac: str = "a4d73ccdaf45") -> bytes:
    """
    Constructs the exact 264-byte auth packet from the PCAP (Frame 118).
    Injects the dynamic Client IP, Projector IP, and MAC address.
    Because the short Rhino Exploit payload forces a socket drop when following 
    the strict registration sequence, this mirrors the exact payload shape.
    """
    hex_ip = get_hex_ip(my_ip)
    hex_proj = get_hex_ip(proj_ip)
    # The PCAP shows `a4d73ccdaf45` (my_mac) in multiple places.
    # We'll just hardcode it or dynamically replace it if passed in.
    p = (
        f"45454d5030313030{hex_ip}01010000f40000000101000000380f000000000000"
        f"ffffff0000000000020f0b0004000320200001ff00ff00ff00000810000000010e0000"
        f"{my_mac}00000000000000000000000000000000192168088001" # Note: we use hardcoded original PCAP internal IP here `c0a85801` but we'll dynamic it:
    )
    # Rebuilding correctly using string formatting
    p = (
        f"45454d5030313030{hex_ip}01010000f40000000101000000380f000000000000"
        f"ffffff0000000000020f0b0004000320200001ff00ff00ff00000810000000010e0000"
        f"{my_mac}00000000000000000000000000000000"
        f"{hex_proj}a600000005000000380000000200000004000000"
        f"{hex_ip}0c00000004000000000000000100000004000000"
        f"500043000b00000004000000000000001c00000000000000040000003600000001000000030000002a000000"
        f"{my_mac}{hex_proj}52455345415243484c4142000000000000000000000000000000000000000000"
        f"0f00000004000000320000000d000000040000000200000026000000080000000010000000100000"
    )
    return binascii.unhexlify(p)

def get_video_init_payload(my_ip: str) -> bytes:
    """
    Constructs the 36-byte EPRD initialization packet for the Video Socket (Port 3621).
    This tells the projector the client is about to send video frames.
    """
    hex_ip = get_hex_ip(my_ip)
    # Magic: EPRD0600 + IP + padding/version (derived from pcap)
    packet_hex = f"4550524430363030{hex_ip}0000000010000000d00000000258a8c00000000000000000"
    return binascii.unhexlify(packet_hex)

def get_pcon_video_header(x_offset: int, y_offset: int, width: int, height: int, payload_length: int) -> bytes:
    """
    Constructs the PCON Video Header to wrap MJPEG stream payloads.
    Total length: 16 bytes.
    """
    magic = b"PCON"
    # Network byte order (Big-Endian) required for PCON
    return struct.pack('>4sHHHHI', magic, x_offset, y_offset, width, height, payload_length)
