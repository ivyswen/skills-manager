import os
import struct
import zlib

def make_png(width, height, r, g, b, a=255):
    # PNG signature
    png = b'\x89PNG\r\n\x1a\n'

    # IHDR chunk
    ihdr_data = struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0)
    ihdr_crc = zlib.crc32(b'IHDR' + ihdr_data) & 0xffffffff
    png += struct.pack('>I', len(ihdr_data)) + b'IHDR' + ihdr_data + struct.pack('>I', ihdr_crc)

    # IDAT chunk (raw scanlines)
    raw_data = bytearray()
    row = bytearray([0]) + bytearray([r, g, b, a] * width) # filter byte 0 + RGBA
    for _ in range(height):
        raw_data.extend(row)

    compressed = zlib.compress(bytes(raw_data))
    idat_crc = zlib.crc32(b'IDAT' + compressed) & 0xffffffff
    png += struct.pack('>I', len(compressed)) + b'IDAT' + compressed + struct.pack('>I', idat_crc)

    # IEND chunk
    iend_crc = zlib.crc32(b'IEND') & 0xffffffff
    png += struct.pack('>I', 0) + b'IEND' + struct.pack('>I', iend_crc)

    return png

def make_ico(png_data, width, height):
    # ICO Header: 2 reserved, 2 type (1 = icon), 2 count (1)
    header = struct.pack('<HHH', 0, 1, 1)
    # Directory entry: width, height, colors, reserved, planes, bpp, size, offset
    w = width if width < 256 else 0
    h = height if height < 256 else 0
    entry = struct.pack('<BBBBHHII', w, h, 0, 0, 1, 32, len(png_data), 6 + 16)
    return header + entry + png_data

out_dir = r"f:\AI\skills-manager\src-tauri\icons"
os.makedirs(out_dir, exist_ok=True)

png_32 = make_png(32, 32, 20, 184, 166, 255) # Teal brand color
png_128 = make_png(128, 128, 20, 184, 166, 255)
ico_data = make_ico(png_32, 32, 32)

with open(os.path.join(out_dir, "32x32.png"), "wb") as f:
    f.write(png_32)

with open(os.path.join(out_dir, "128x128.png"), "wb") as f:
    f.write(png_128)

with open(os.path.join(out_dir, "128x128@2x.png"), "wb") as f:
    f.write(png_128)

with open(os.path.join(out_dir, "icon.ico"), "wb") as f:
    f.write(ico_data)

with open(os.path.join(out_dir, "icon.icns"), "wb") as f:
    # Dummy icns
    f.write(b'icns\x00\x00\x00\x10' + png_32[:8])

print("Icons generated successfully!")
