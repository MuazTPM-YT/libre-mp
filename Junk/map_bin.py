import struct
import os

if not os.path.exists("windows_perfect_stream.bin"):
    print("[-] Could not find windows_perfect_stream.bin!")
    exit()

with open("windows_perfect_stream.bin", "rb") as f:
    data = f.read()

print(f"[*] Analyzing Windows Stream: {len(data)} total bytes\n")

offset = 0
batch_count = 0

while offset < len(data):
    # Find the next EPRD header
    eprd_idx = data.find(b'EPRD0600', offset)
    
    if eprd_idx == -1:
        break

    # CRITICAL: Check for any secret bytes between the end of the last frame and the start of this one
    if eprd_idx > offset and batch_count > 0:
        gap = data[offset:eprd_idx]
        print(f"\n[!] CRITICAL DISCOVERY: Found {len(gap)} bytes of hidden data between batches!")
        print(f"[!] Hex: {gap.hex()}\n")

    header = data[eprd_idx:eprd_idx+20]
    if len(header) < 20: break

    # Extract sizes (Testing both Endiannesses)
    size_bytes = header[16:20]
    size_be = struct.unpack('>I', size_bytes)[0]
    size_le = struct.unpack('<I', size_bytes)[0]

    if size_le == 46:
        block_type = "META CONFIG"
        actual_size = size_le
    else:
        block_type = "VIDEO BATCH"
        actual_size = size_be # Video batches use Big-Endian size

    print(f"--- EPRD Block {batch_count+1} : {block_type} ---")
    print(f"Declared Payload Size: {actual_size} bytes")

    payload_start = eprd_idx + 20
    payload_end = payload_start + actual_size
    
    # Analyze what is inside this specific batch
    payload_data = data[payload_start:payload_end]
    jpeg_count = payload_data.count(b'\xff\xd8')
    
    if block_type == "VIDEO BATCH":
        print(f"-> Contains {jpeg_count} JPEG tiles.")
        
        # Look at the first 4 bytes of the payload
        first_4 = payload_data[:4]
        if len(first_4) == 4:
            print(f"-> Starts with bytes: {first_4.hex()} (Expected Frame Type 00000004)")

    offset = payload_end
    batch_count += 1
    
    if batch_count >= 5: # Just map the first 5 blocks
        break