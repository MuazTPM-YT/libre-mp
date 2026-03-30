import struct
import sys
import os

def dump_video_stream(pcap_file):
    print(f"[*] Reading {pcap_file}...")
    if not os.path.exists(pcap_file):
        print(f"[-] Could not find {pcap_file}. Please check the path!")
        return

    try:
        with open(pcap_file, 'rb') as f: data = f.read()
    except Exception as e:
        print(f"[-] Error: {e}")
        return
    
    streams = {} 
    pos = 0
    while pos + 12 <= len(data):
        btype = struct.unpack('<I', data[pos:pos+4])[0]
        blen = struct.unpack('<I', data[pos+4:pos+8])[0]
        if blen < 12 or pos + blen > len(data): break
        bbody = data[pos+8:pos+blen-4]
        if btype == 6:  # EPB (Enhanced Packet Block)
            caplen = struct.unpack('<I', bbody[12:16])[0]
            pkt = bbody[20:20+caplen]
            if len(pkt) >= 54 and struct.unpack('>H', pkt[12:14])[0] == 0x0800:
                ip = pkt[14:]
                if ip[9] == 6 and len(ip) >= 20: # TCP
                    ihl = (ip[0] & 0x0F) * 4
                    tcp = ip[ihl:]
                    if len(tcp) >= 20:
                        sport = struct.unpack('>H', tcp[0:2])[0]
                        dport = struct.unpack('>H', tcp[2:4])[0]
                        if dport == 3621:
                            doff = ((tcp[12] >> 4) & 0x0F) * 4
                            payload = tcp[doff:struct.unpack('>H', ip[2:4])[0] - ihl]
                            if payload:
                                flow = (sport, dport)
                                if flow not in streams:
                                    streams[flow] = bytearray()
                                streams[flow].extend(payload)
        pos += blen
        
    for flow, payload in streams.items():
        # Look for the stream that starts with EPRD0600 and byte 28 is 0x00 (Video Socket)
        if len(payload) > 36 and payload[:8] == b'EPRD0600' and payload[28] == 0x00:
            print(f"[*] Found the true 0x00 Video Stream! Total size: {len(payload)} bytes.")
            with open("windows_perfect_stream.bin", "wb") as f:
                # We strip the first 36 bytes (the Init Block) because _open_video_channels() already sends it!
                f.write(payload[36:])
            print("[+] Saved payload to 'windows_perfect_stream.bin'")
            return
            
    print("[-] Could not find the video stream in the PCAP.")

# IMPORTANT: Ensure this points to your actual pcapng file!
dump_video_stream('tls.pcapng')