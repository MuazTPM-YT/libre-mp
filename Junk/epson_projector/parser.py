import binascii
from collections import Counter

def parse_payload(hex_payload: str):
    raw_bytes = bytes.fromhex(hex_payload)
    video_stream = raw_bytes[16:]

    # Look at repeating 16-byte sequences (could indicate ECB mode or padding)
    blocks = [video_stream[i:i+16] for i in range(8, len(video_stream), 16) if i+16 <= len(video_stream)]
    counter = Counter(blocks)
    for block, count in counter.most_common(5):
        print(f"{count} times: {binascii.hexlify(block).decode()}")
