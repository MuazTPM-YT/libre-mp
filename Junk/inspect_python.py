import struct
import os

if not os.path.exists("python_perfect_stream.bin"):
    print("[-] Could not find python_perfect_stream.bin!")
    exit()

with open("python_perfect_stream.bin", "rb") as f:
    data = f.read()

# 1. Carve the JPEG out of the stream
start = data.find(b'\xff\xd8')
if start == -1:
    print("[-] Could not find JPEG start marker (FF D8)")
    exit()

end = data.find(b'\xff\xd9', start)
if end != -1:
    end += 2 # include the FF D9 marker
else:
    end = len(data)

jpeg_data = data[start:end]

# 2. Save it so you can see what Windows actually sent
with open("python_frame1.jpg", "wb") as f:
    f.write(jpeg_data)

print(f"[+] Successfully extracted Windows JPEG! ({len(jpeg_data)} bytes)")
print(f"[!] Please open 'python_frame1.jpg' on your machine. What is it an image of?")
print("-" * 50)

# 3. Parse the JPEG structure to find what Python/Pillow is doing differently
print("--- Windows JPEG Internal Architecture ---")
idx = 2
while idx < len(jpeg_data):
    marker_bytes = jpeg_data[idx:idx+2]
    if len(marker_bytes) < 2: break
    marker = struct.unpack('>H', marker_bytes)[0]
    idx += 2
    
    if marker == 0xFFDA: # Start of Scan (Image Data Begins)
        print(f"[{hex(marker)}] SOS (Start of Scan) -> Pixel data begins here.")
        break
        
    if (marker & 0xFF00) == 0xFF00:
        length = struct.unpack('>H', jpeg_data[idx:idx+2])[0]
        
        name = "Unknown"
        if marker == 0xFFC0: name = "SOF0 (Baseline)"
        elif marker == 0xFFC2: name = "SOF2 (Progressive)"
        elif marker == 0xFFDB: name = "DQT (Quantization Table)"
        elif marker == 0xFFC4: name = "DHT (Huffman Table)"
        elif marker == 0xFFE0: name = "APP0 (JFIF Header)"
        elif marker == 0xFFE1: name = "APP1 (EXIF)"
        
        print(f"[{hex(marker)}] {name} - Length: {length} bytes")
        
        if marker == 0xFFC0: # Parse the crucial SOF0 metadata
            precision = jpeg_data[idx+2]
            h, w = struct.unpack('>HH', jpeg_data[idx+3:idx+7])
            comps = jpeg_data[idx+7]
            print(f"    -> Resolution: {w}x{h}")
            print(f"    -> Precision: {precision}-bit")
            print(f"    -> Color Components: {comps}")
            for i in range(comps):
                cid = jpeg_data[idx+8 + i*3]
                samp = jpeg_data[idx+9 + i*3]
                hq = samp >> 4
                vq = samp & 0x0F
                qt = jpeg_data[idx+10 + i*3]
                print(f"       Channel {cid}: H-Subsampling={hq}, V-Subsampling={vq}, QuantTable={qt}")
                
        idx += length
    else:
        print(f"[-] Hit unknown marker or corrupted data: {hex(marker)}")
        break