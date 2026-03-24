import socket
import struct
import binascii
import time

# ========================================================================
#  Helpers
# ========================================================================

def get_hex_ip(ip: str) -> str:
    return socket.inet_aton(ip).hex()

def get_hex_ip_reversed(ip: str) -> str:
    return socket.inet_aton(ip)[::-1].hex()

# ========================================================================
#  Port 3620 – Authentication
# ========================================================================

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
    """
    hex_ip = get_hex_ip(my_ip)
    hex_proj = get_hex_ip(proj_ip)
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

# ========================================================================
#  Port 3621 – Video Channel (EPRD Protocol)
# ========================================================================

def get_video_init_payload_ctrl(my_ip: str) -> bytes:
    """
    36-byte EPRD init for the CONTROL video connection (first TCP to 3621).
    Byte 28 = 0x00 — marks this as the control channel.
    Derived from PCAP Frame 138.
    """
    hex_ip = get_hex_ip(my_ip)
    hex_ip_rev = get_hex_ip_reversed(my_ip)
    p = f"4550524430363030{hex_ip}0000000010000000d0000000{hex_ip_rev}0000000000000000"
    return binascii.unhexlify(p)

def get_video_init_payload_data(my_ip: str) -> bytes:
    """
    36-byte EPRD init for the DATA video connection (second TCP to 3621).
    Byte 28 = 0x01 — marks this as the data/streaming channel.
    Derived from PCAP Frame 143.
    """
    hex_ip = get_hex_ip(my_ip)
    hex_ip_rev = get_hex_ip_reversed(my_ip)
    p = f"4550524430363030{hex_ip}0000000010000000d0000000{hex_ip_rev}0100000000000000"
    return binascii.unhexlify(p)

def get_aux_header(size: int) -> bytes:
    """
    Returns only the 5-byte header for auxiliary packets (0xC9 + LE_uint32 length).
    Used to force specific TCP segmentation matching the Windows client.
    """
    return struct.pack('<BI', 0xC9, size)

def get_zero_buffer(size: int) -> bytes:
    """
    Build a zero-padded warmup buffer with the 5-byte 0xC9 length prefix.
    PCAP shows three of these sent before any video data:
      - 7276 bytes, 2646 bytes, 1764 bytes (all zeros)
    Format: 0xC9 + LE_uint32(size) + size bytes of 0x00
    """
    return get_aux_header(size) + (b'\x00' * size)

def get_eprd_meta_header(my_ip: str, meta_size: int) -> bytes:
    """
    Build a 20-byte EPRD0600 header for display config metadata.
    Size field is Little-Endian (pcap: 0x2e000000 = LE 46).
    """
    ip_bytes = socket.inet_aton(my_ip)
    return b'EPRD0600' + ip_bytes + struct.pack('<II', 0, meta_size)

def get_eprd_jpeg_header(my_ip: str, jpeg_size: int) -> bytes:
    """
    Build a 20-byte EPRD0600 header for JPEG frame data.
    Size field is Big-Endian (pcap: 0x00012af5 = BE 76533).
    """
    ip_bytes = socket.inet_aton(my_ip)
    return b'EPRD0600' + ip_bytes + struct.pack('>II', 0, jpeg_size)

# get_eprd_init_block is NOT needed during frame sending.
# The channel is already initialized by get_video_init_payload_ctrl() during _open_video_channels().
# Windows PCAP confirms: no extra init block before the first frame's meta/JPEG data.
    
def get_display_config_meta(disp_w: int = 1600, disp_h: int = 900, stream_w: int = 624, stream_h: int = 416) -> bytes:
    """
    Build the 46-byte display configuration block sent on the first frame.
    Starts with 0xCC. Contains display resolution and color info.
    Derived from PCAP Frame 183.
    
    PCAP exact bytes (tls.pcapng reassembled):
    cc0000000400030020200001ff00ff00ff0010080000000006400384000000600400024000000000000000000000
    
    Rather than guessing padding, endianness, and unknown bit flags, we use exactly what Windows sends.
    """
    # Simply hardcode the exact 46 bytes sent by Windows.
    # The resolution 1600x900 is baked in as 06 40 03 84 implicitly by the PCAP capture.
    return bytes.fromhex("cc0000000400030020200001ff00ff00ff0010080000000006400384000000600400024000000000000000000000")

def get_frame_header(frame_type: int, x: int, y: int, w: int, h: int) -> bytes:
    """
    Build the 20-byte frame header: 4-byte type (BE) + 16-byte region descriptor.
    
    Layout: type(u32 BE) + x(u16 BE) + y(u16 BE) + w(u16 BE) + h(u16 BE) + flags(u32 BE) + timestamp(u32 BE)
    
    PCAP (tls.pcapng) first frame: 00000004 0000 0000 0270 01a0 00000007 90d48902
      type=4, x=0, y=0, w=624, h=416, flags=0x07, ts=rolling
    """
    ts = int(time.time() * 1000) & 0xFFFFFFFF
    return struct.pack('>I', frame_type) + struct.pack('>HHHH', x, y, w, h) + struct.pack('>II', 0x00000007, ts)

def get_subsequent_frame_header(x: int, y: int, w: int, h: int) -> bytes:
    """
    Build the 16-byte subsequent frame header (NO type prefix, NO EPRD header).
    
    PCAP (tls.pcapng) shows subsequent frames have only:
      x(u16 BE), y(u16 BE), w(u16 BE), h(u16 BE), flags(u32 BE), timestamp(u32 BE)
    
    Example from pcap: 02700000019001a00000000790d8df00
      x=624, y=0, w=400, h=416, flags=0x07, ts=rolling
    """
    ts = int(time.time() * 1000) & 0xFFFFFFFF
    region = struct.pack('>HHHH', x, y, w, h)
    tail = struct.pack('>II', 0x00000007, ts)
    return region + tail


# ========================================================================
#  Single-buffer frame assembly (matching Windows PCAP behavior)
#
#  The Epson projector firmware expects the EPRD header and its payload
#  to arrive together in the TCP stream. Sending them as separate
#  sendall() calls with TCP_NODELAY may cause the projector to reject
#  or ignore the frame.
# ========================================================================

def build_first_frame_payload(my_ip: str, disp_w: int, disp_h: int,
                              x: int, y: int, w: int, h: int,
                              jpeg_bytes: bytes) -> bytes:
    meta = get_display_config_meta(disp_w, disp_h)
    meta_hdr = get_eprd_meta_header(my_ip, len(meta))

    frame_hdr = get_frame_header(4, x, y, w, h)
    jpeg_payload_size = len(frame_hdr) + len(jpeg_bytes)
    jpeg_hdr = get_eprd_jpeg_header(my_ip, jpeg_payload_size)

    return meta_hdr + meta + jpeg_hdr + frame_hdr + jpeg_bytes

def build_video_frame_payload(my_ip: str, frame_type: int,
                              x: int, y: int, w: int, h: int,
                              jpeg_bytes: bytes) -> bytes:
    """
    Assemble a subsequent video frame as a single contiguous buffer:
      [EPRD jpeg header (20)] [Frame header (20)] [JPEG data]
    """
    frame_hdr = get_frame_header(frame_type, x, y, w, h)
    jpeg_payload_size = len(frame_hdr) + len(jpeg_bytes)
    jpeg_hdr = get_eprd_jpeg_header(my_ip, jpeg_payload_size)

    return jpeg_hdr + frame_hdr + jpeg_bytes
