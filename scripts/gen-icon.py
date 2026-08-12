import struct, zlib

W = H = 1024
color = (30, 41, 59)  # slate-800
raw = b"".join(b"\x00" + bytes(color) * W for _ in range(H))

def chunk(tag, data):
    c = tag + data
    return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)

png = b"\x89PNG\r\n\x1a\n"
png += chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0))
png += chunk(b"IDAT", zlib.compress(raw, 9))
png += chunk(b"IEND", b"")

with open("scripts/app-icon.png", "wb") as f:
    f.write(png)
print("wrote scripts/app-icon.png")