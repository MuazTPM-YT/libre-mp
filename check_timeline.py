import struct

def check_timeline(filename):
    with open(filename, 'rb') as f: data = f.read()
    events = []
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
                    src_ip = ".".join(map(str, ip[12:16]))
                    dst_ip = ".".join(map(str, ip[16:20]))
                    ihl = (ip[0] & 0x0F) * 4
                    tcp = ip[ihl:]
                    if len(tcp) >= 20:
                        sp = struct.unpack('>H', tcp[0:2])[0]
                        dp = struct.unpack('>H', tcp[2:4])[0]
                        
                        doff = ((tcp[12] >> 4) & 0x0F) * 4
                        payload_len = struct.unpack('>H', ip[2:4])[0] - ihl - doff
                        if payload_len > 0:
                            payload = tcp[doff:doff+payload_len]
                            events.append((ts, src_ip, dst_ip, sp, dp, payload))
        pos += blen
        
    # Sort purely by Timestamp to get EXACT network timeline
    events.sort(key=lambda x: x[0])
    
    # First identify the exact ports
    video_port = None
    aux_port = None
    for (ts, src, dst, sp, dp, pay) in events:
        if dp == 3621 and pay.startswith(b'EPRD0600') and len(pay) > 28:
            if pay[28] == 0x00: video_port = sp
            if pay[28] == 0x01: aux_port = sp

    print(f"[*] Detected Video Port: {video_port}")
    print(f"[*] Detected Aux Port: {aux_port}")

    for i, (ts, src, dst, sp, dp, pay) in enumerate(events):
        if sp not in (video_port, aux_port) and dp not in (video_port, aux_port):
            continue

        if dp == 3621:
            if sp == video_port:
                if pay.startswith(b'EPRD0600'):
                    print(f"[{i:04d}] [TS: {ts}] Client({sp}) -> Proj(3621) | VIDEO EPRD | len={len(pay)} | hex={pay[:16].hex()}")
                else:
                    print(f"[{i:04d}] [TS: {ts}] Client({sp}) -> Proj(3621) | VIDEO DATA | len={len(pay)} | hex={pay[:16].hex()}")
            elif sp == aux_port:
                if pay.startswith(b'EPRD0600'):
                    print(f"[{i:04d}] [TS: {ts}] Client({sp}) -> Proj(3621) | AUX EPRD | len={len(pay)} | hex={pay[:16].hex()}")
                elif pay.startswith(b'\\xc9'):
                    print(f"[{i:04d}] [TS: {ts}] Client({sp}) -> Proj(3621) | AUX HEADER | len={len(pay)} | hex={pay[:16].hex()}")
                else:
                    print(f"[{i:04d}] [TS: {ts}] Client({sp}) -> Proj(3621) | AUX DATA | len={len(pay)} | hex={pay[:16].hex()}")
        elif sp == 3621:
            if dp == video_port:
                print(f"[{i:04d}] [TS: {ts}] Proj(3621) -> Client({dp}) | VIDEO ACK | len={len(pay)}")
            elif dp == aux_port:
                print(f"[{i:04d}] [TS: {ts}] Proj(3621) -> Client({dp}) | AUX ACK | len={len(pay)}")
                
        # Also print Port 3620 (Control) so we see the 0x010C and 0x0016
        if dp == 3620:
            if pay.startswith(b'EMP0'):
                pass # Registration
            elif struct.unpack('>H', pay[:2])[0] == 0x010C:
                print(f"[{i:04d}] [TS: {ts}] Client -> Proj(3620) | 0x010C PLAY COMMAND | len={len(pay)}")
        elif sp == 3620:
            if len(pay) >= 2 and struct.unpack('>H', pay[:2])[0] == 0x0016:
                print(f"[{i:04d}] [TS: {ts}] Proj(3620) -> Client | 0x0016 STREAM READY | len={len(pay)}")

check_timeline('tls.pcapng')
