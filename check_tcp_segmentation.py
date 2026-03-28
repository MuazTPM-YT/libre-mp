import struct

def check_segmentation(filename):
    with open(filename, 'rb') as f: data = f.read()
    c2p = []
    pos = 0
    while pos + 12 <= len(data):
        btype = struct.unpack('<I', data[pos:pos+4])[0]
        blen = struct.unpack('<I', data[pos+4:pos+8])[0]
        if blen < 12 or pos + blen > len(data): break
        bbody = data[pos+8:pos+blen-4]
        if btype == 6:  # EPB
            ts_high = struct.unpack('<I', bbody[4:8])[0]
            ts_low = struct.unpack('<I', bbody[8:12])[0]
            ts = (ts_high << 32) | ts_low
            
            caplen = struct.unpack('<I', bbody[12:16])[0]
            pkt = bbody[20:20+caplen]
            if len(pkt) >= 54 and struct.unpack('>H', pkt[12:14])[0] == 0x0800:
                ip = pkt[14:]
                if ip[9] == 6 and len(ip) >= 20:
                    ihl = (ip[0] & 0x0F) * 4
                    tcp = ip[ihl:]
                    if len(tcp) >= 20:
                        dp = struct.unpack('>H', tcp[2:4])[0]
                        if dp == 3621:
                            doff = ((tcp[12] >> 4) & 0x0F) * 4
                            payload = tcp[doff:struct.unpack('>H', ip[2:4])[0] - ihl]
                            if payload:
                                c2p.append((ts, payload))
        pos += blen
    
    for i, (ts, pay) in enumerate(c2p):
        hex_start = pay[:16].hex()
        # Look for the start of the EPRD meta header
        if hex_start.startswith('4550524430363030') and len(pay) != 36:
            print(f"\\n--- First Frame Stream Starts at Packet {i+1} ---")
            for j in range(6):
                if i+j < len(c2p):
                    print(f"Packet {i+j+1}: length {len(c2p[i+j][1])}")
            break

check_segmentation('tls.pcapng')
