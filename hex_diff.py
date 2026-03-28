import sys

def hex_diff(file1, file2):
    with open(file1, 'rb') as f1, open(file2, 'rb') as f2:
        d1 = f1.read()
        d2 = f2.read()

    print(f"File 1 ({file1}): {len(d1)} bytes")
    print(f"File 2 ({file2}): {len(d2)} bytes")

    cmp_len = min(len(d1), len(d2))
    
    # Let's find the first mismatch
    mismatch_idx = -1
    for i in range(cmp_len):
        if d1[i] != d2[i]:
            # we know the timestamps (which vary) are at specific positions.
            # first tile timestamp is at 102 (from the batch payload start -> 16 bytes per region = 12 bytes + 4 bytes timestamp)
            # Actually, let's just ignore the expected timestamp offsets.
            # Meta config + headers is 66 + 20 + 4 = 90 bytes. 
            # Region 1 is 90 to 106. Timestamp is 90 + 12 = 102.
            # 102, 103, 104, 105.
            if i in [138, 139, 140, 141]: # from compare_dump.py!
                continue
                
            mismatch_idx = i
            break
            
    if mismatch_idx != -1:
        print(f"\\nMismatch found at index {mismatch_idx} (0x{mismatch_idx:X})")
        
        # print context
        start = max(0, mismatch_idx - 32)
        end = min(cmp_len, mismatch_idx + 32)
        
        print(f"\\nContext around mismatch ({start} to {end}):")
        print("WIN: ", d1[start:end].hex(' '))
        print("PYT: ", d2[start:end].hex(' '))
        
        # point out the exact byte with a caret
        caret = " " * ((mismatch_idx - start) * 3)
        print("     " + caret + "^")
    else:
        print("\\nNo mismatches found up to the common length (ignoring timestamps)!")

hex_diff('windows_perfect_stream.bin', 'python_perfect_stream.bin')
