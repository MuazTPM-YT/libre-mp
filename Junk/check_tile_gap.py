import struct

with open('windows_perfect_stream.bin', 'rb') as f:
    d = f.read()

meta_len = struct.unpack('<I', d[16:20])[0]
meta_end = 20 + meta_len
batch_len = struct.unpack('>I', d[meta_end+16:meta_end+20])[0]

vnc_start = meta_end + 20
tile_count = struct.unpack('>I', d[vnc_start:vnc_start+4])[0]

pos = vnc_start + 4
for i in range(tile_count):
    x, y, w, h = struct.unpack('>HHHH', d[pos:pos+8])
    flags = struct.unpack('>I', d[pos+8:pos+12])[0]
    ts = struct.unpack('>I', d[pos+12:pos+16])[0]
    print(f"Tile {i+1}: X={x}, Y={y}, W={w}, H={h}, Flags={flags}, TS={ts}")
    pos += 16
    
    # The JPEG starts here
    if d[pos:pos+2] != b'\xff\xd8':
        print(f"WARNING: Tile {i+1} doesn't start with JPEG SOI! Found {d[pos:pos+2].hex()}")
        
    jpeg_end = d.find(b'\xff\xd9', pos) + 2
    jpeg_size = jpeg_end - pos
    print(f"   JPEG Size: {jpeg_size}")
    
    pos = jpeg_end
    
    # Check what is immediately after the JPEG!
    if i < tile_count - 1:
        # The next thing should be the next region descriptor!
        # Let's peek the next 16 bytes
        next_16 = d[pos:pos+16]
        # Does it look like a valid region descriptor?
        nx, ny, nw, nh = struct.unpack('>HHHH', next_16[:8])
        print(f"   Gap check: Next 16 bytes interpret as X={nx}, Y={ny}, W={nw}, H={nh}")
        # If there is a gap, what are the bytes?
        # Actually, let's just see if d.find(b'\xff\xd8', pos) points to exactly pos + 16!
        next_jpeg_start = d.find(b'\xff\xd8', pos)
        gap = next_jpeg_start - (pos + 16)
        if gap > 0:
            print(f"   [!] Found {gap} bytes of PADDING before next Tile header!")
            print(f"   Padding hex: {d[pos:pos+gap].hex()}")
            pos += gap # adjust pos so the loop reads the actual region descriptor next
        elif gap < 0:
            print(f"   [!] Next JPEG starts BEFORE the 16 byte header???")
            
