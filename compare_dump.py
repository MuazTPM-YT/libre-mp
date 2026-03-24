#!/usr/bin/env python3
"""Compare the locally dumped video_stream_debug.bin perfectly against the Windows PCAP stream."""

import struct
import sys
import os

def read_tcp(filename, target_port=3621):
    try:
        with open(filename, 'rb') as f: data = f.read()
    except: return b''
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

win = read_tcp('/home/iffelse/libre-mp/pcap/tls.pcapng')
if not win:
    print("Could not load Windows PCAP.")
    sys.exit(1)

dump_path = '/home/iffelse/libre-mp/video_stream_debug.bin'
if not os.path.exists(dump_path):
    print(f"Dump file {dump_path} not found. Run the client first.")
    sys.exit(0)

with open(dump_path, 'rb') as f:
    dump = f.read()

print(f"Windows PCAP stream size: {len(win)} bytes")
print(f"Local dump size:          {len(dump)} bytes")

# We expect the dump to match the PCAP exactly, except for:
# 1. 24 bytes in (display resolution at offset 24 might differ if we passed different args than 1600x900)
# 2. 122+12-15 bytes in (timestamp in first frame header)
# 3. JPEG payload (might be compressed slightly differently by PIL vs Windows UI)

cmp_len = min(142, min(len(win), len(dump)))
diffs = 0

print(f"\n--- Comparing first {cmp_len} protocol bytes ---")
for i in range(cmp_len):
    if win[i] != dump[i]:
        # Ignore timestamp field difference (Offset 138-141)
        if 138 <= i <= 141:
            continue
            
        diffs += 1
        print(f"DIFF at index {i:03d} (0x{i:02x}): WIN=0x{win[i]:02x} DUMP=0x{dump[i]:02x}")

if diffs == 0:
    print("\nSUCCESS! The entire 142-byte protocol header chain matches Windows PERFECTLY (ignoring variable timestamp).")
    print("If it still resets, the issue MUST be in the actual JPEG payload contents or fragmentation/timing.")
else:
    print(f"\nFAILED! Found {diffs} protocol mismatches before the JPEG data even begins.")

print("\nWIN 0-142:", win[:142].hex())
print("DMP 0-142:", dump[:142].hex())
