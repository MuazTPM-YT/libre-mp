import struct
import sys

def read_tcp(filename, target_port=3621):
    try:
        with open(filename, 'rb') as f: data = f.read()
    except Exception as e:
        print(f"[-] Error reading {filename}: {e}")
        return b''
    c2p = []
    pos = 0
    while pos + 12 <= len(data):
        btype = struct.unpack('<I', data[pos:pos+4])[0]
        blen = struct.unpack('<I', data[pos+4:pos+8])[0]
        if blen < 12 or pos + blen > len(data): break
        bbody = data[pos+8:pos+blen-4]
        if btype == 6:  # EPB
            caplen = struct.unpack('<I', bbody[12:16])[0]
            pkt = bbody[20:20+caplen]
            if len(pkt) >= 54 and struct.unpack('>H', pkt[12:14])[0] == 0x0800:
                ip = pkt[14:]
                if ip[9] == 6 and len(ip) >= 20:
                    ihl = (ip[0] & 0x0F) * 4
                    tcp = ip[ihl:]
                    if len(tcp) >= 20:
                        dp = struct.unpack('>H', tcp[2:4])[0]
                        if dp == target_port:
                            doff = ((tcp[12] >> 4) & 0x0F) * 4
                            payload = tcp[doff:struct.unpack('>H', ip[2:4])[0] - ihl]
                            if payload:
                                c2p.append((struct.unpack('>I', tcp[4:8])[0], payload))
        pos += blen
    c2p.sort(key=lambda x: x[0])
    
    res = []
    curr = bytearray()
    last_seq = -1
    for seq, pay in c2p:
        if last_seq != -1 and seq > last_seq + 100000:
            if curr: res.append(curr)
            curr = bytearray()
        if seq > last_seq:
            curr.extend(pay)
            last_seq = seq + len(pay)
    if curr: res.append(curr)
    
    for r in res:
        if len(r) > 36 and r[28] == 0x00: return r
    return b''

print("[*] Parsing Windows PCAP stream...")
# Make sure this points to your actual Windows pcapng file!
win_data = read_tcp('tls.pcapng') 

if not win_data:
    print("[-] Failed to load PCAP data. Check the path!")
    sys.exit(1)

offset = 0
frame_count = 0

while True:
    idx = win_data.find(b'EPRD0600', offset)
    if idx == -1:
        break
        
    frame_count += 1
    header = win_data[idx:idx+20]
    size = struct.unpack('>I', header[16:20])[0]
    
    print(f"\n--- EPRD Block {frame_count} ---")
    print(f"Declared Payload Size: {size} bytes")
    
    payload_start = idx + 20
    # Search for the JPEG Start Marker (FF D8)
    jpeg_idx = win_data.find(b'\xff\xd8', payload_start, payload_start + 100)
    
    if jpeg_idx != -1:
        dist = jpeg_idx - payload_start
        print(f"Header length before JPEG: {dist} bytes")
        print(f"Header Hex: {win_data[payload_start:jpeg_idx].hex()}")
        
        # Look at the first 16 bytes of the JPEG
        jpeg_sample = win_data[jpeg_idx:jpeg_idx+16]
        print(f"JPEG Start Hex: {jpeg_sample.hex()}")
    else:
        print("No JPEG found in this block (Likely the Meta Config Block).")
        
    offset = idx + 20
    if frame_count >= 5: # We only need to see the first few frames
        break