import struct

with open('windows_perfect_stream.bin', 'rb') as f:
    d = f.read()

# Skip Meta Block length
meta_len = struct.unpack('<I', d[16:20])[0]
print(f"Meta length: {meta_len}")
meta_end = 20 + meta_len
print(f"EPRD Batch starts at {meta_end}")

if d[meta_end:meta_end+8] != b'EPRD0600':
    print("Invalid Batch Header")

batch_len = struct.unpack('>I', d[meta_end+16:meta_end+20])[0]
print(f"Batch Length: {batch_len}")

vnc_start = meta_end + 20
tile_count = struct.unpack('>I', d[vnc_start:vnc_start+4])[0]
print(f"Tile Count: {tile_count}")

# Parse tiles
pos = vnc_start + 4
for i in range(tile_count):
    x, y, w, h = struct.unpack('>HHHH', d[pos:pos+8])
    flags = struct.unpack('>I', d[pos+8:pos+12])[0]
    ts = struct.unpack('>I', d[pos+12:pos+16])[0]
    print(f"Tile {i+1}: X={x}, Y={y}, W={w}, H={h}, Flags={flags}, TS={ts}")
    pos += 16
    
    # Need to skip the JPEG. The JPEG ends or starts with FF D8... FF D9
    if d[pos:pos+2] != b'\xff\xd8':
        print(f"WARNING: Tile {i+1} doesn't start with JPEG SOI ({d[pos:pos+2].hex()})")
        
    jpeg_end = d.find(b'\xff\xd9', pos) + 2
    jpeg_size = jpeg_end - pos
    print(f"   JPEG Size: {jpeg_size}")
    pos = jpeg_end
