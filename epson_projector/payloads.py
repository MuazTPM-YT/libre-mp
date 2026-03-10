import socket

def get_hex_ip(ip: str) -> str:
    return socket.inet_aton(ip).hex()

def get_hex_ip_reversed(ip: str) -> str:
    return socket.inet_aton(ip)[::-1].hex()

def get_control_payload(my_ip: str) -> bytes:
    hex_ip = get_hex_ip(my_ip)
    # Payload from original epson_client.py (Frame 235 Data: EEMP0100 68 bytes)
    # Reverting to 10010000... payload as requested in user notes.
    return bytes.fromhex(
        f"45454d5030313030{hex_ip}1001000000000000"
        "0000000000000000000000000000000000000000"
        "0000000000000000000000000000000000000000"
    )

def get_control_payload_phase_1_part_2(my_ip: str) -> bytes:
    hex_ip = get_hex_ip(my_ip)
    # Frame 40 (94 bytes)
    # The original Windows IP was 192.168.88.2 (c0a85802)
    return bytes.fromhex(
        f"45454d5030313030{hex_ip}030000004a000000"
        "0100000052455345415243484c41420000000000"
        "0000000000000000000000000000000000000100"
        "0b000001000008000000a4d73ccdaf4500000000"
        "00000000000000000000000000000000"
    )

def get_control_payload_phase_2(my_ip: str, projector_ip: str) -> bytes:
    hex_my_ip = get_hex_ip(my_ip)
    hex_proj_ip = get_hex_ip(projector_ip)
    # Frame 75 (264 bytes)
    # Original Windows IP: 192.168.88.2 (c0a85802) -> hex_my_ip
    # Original Proj IP: 192.168.88.1 (c0a85801) -> hex_proj_ip
    return bytes.fromhex(
        f"45454d5030313030{hex_my_ip}01010000f4000000"
        "0101000000fc04000000000000ffffff00000000"
        "00020f0b0004000320200001ff00ff00ff000008"
        "10000000010e0000a4d73ccdaf45000000000000"
        f"00000000000000000000{hex_proj_ip}a6000000"
        f"05000000380000000200000004000000{hex_my_ip}"
        "0c00000004000000000000000100000004000000"
        "500043000b00000004000000000000001c000000"
        "0000000004000000360000000100000003000000"
        f"2a000000a4d73ccdaf45{hex_proj_ip}52455345"
        "415243484c414200000000000000000000000000"
        "00000000000000000f0000000400000032000000"
        "0d00000004000000020000002600000008000000"
        "0010000000100000"
    )

def get_video_payload(my_ip: str) -> bytes:
    hex_ip = get_hex_ip(my_ip)
    hex_ip_reversed = get_hex_ip_reversed(my_ip)
    # Frame 237/238 type EPRD0600 - The Video Handshake
    return bytes.fromhex(
        f"4550524430363030{hex_ip}0000000010000000"
        f"d0000000{hex_ip_reversed}0000000000000000"
    )

def get_wakeup_payload() -> bytes:
    # State Trigger
    return bytes.fromhex("4553432f56502e6e6574200200000001060100000000000000000000000000000000")
