with open("windows_perfect_stream.bin", "rb") as f:
    data = f.read()

# Find First JPEG
jpeg1_start = data.find(b'\xff\xd8')
jpeg1_end = data.find(b'\xff\xd9', jpeg1_start) + 2

print("--- FIRST FRAME HEADER ---")
header1 = data[:jpeg1_start]
print(f"Length: {len(header1)} bytes")
print(f"Hex: {header1.hex()}")

# Find Second JPEG
jpeg2_start = data.find(b'\xff\xd8', jpeg1_end)
if jpeg2_start != -1:
    print("\n--- GAP BETWEEN FRAME 1 & FRAME 2 ---")
    gap = data[jpeg1_end:jpeg2_start]
    print(f"Length: {len(gap)} bytes")
    print(f"Hex: {gap.hex()}")
else:
    print("\nNo second frame found.")