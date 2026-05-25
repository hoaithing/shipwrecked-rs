import os
import zlib
import struct

def write_grayscale_png(filename, width, height, data):
    png = b'\x89PNG\r\n\x1a\n'
    
    ihdr_data = struct.pack('>IIBBBBB', width, height, 8, 0, 0, 0, 0)
    ihdr = b'IHDR' + ihdr_data
    png += struct.pack('>I', len(ihdr_data)) + ihdr + struct.pack('>I', zlib.crc32(ihdr))
    
    scanlines = []
    for y in range(height):
        scanlines.append(b'\x00' + data[y * width : (y + 1) * width])
    raw_data = b''.join(scanlines)
    idat_data = zlib.compress(raw_data)
    idat = b'IDAT' + idat_data
    png += struct.pack('>I', len(idat_data)) + idat + struct.pack('>I', zlib.crc32(idat))
    
    iend = b'IEND'
    png += struct.pack('>I', 0) + iend + struct.pack('>I', zlib.crc32(iend))
    
    with open(filename, 'wb') as f:
        f.write(png)

def read_grayscale_png(filename):
    with open(filename, 'rb') as f:
        png = f.read()
    if png[:8] != b'\x89PNG\r\n\x1a\n':
        raise ValueError("Not a PNG file")
    
    idx = 8
    width = height = 0
    idat_data = []
    while idx < len(png):
        length = struct.unpack('>I', png[idx:idx+4])[0]
        chunk_type = png[idx+4:idx+8]
        chunk_data = png[idx+8:idx+8+length]
        idx += 12 + length
        
        if chunk_type == b'IHDR':
            width, height = struct.unpack('>II', chunk_data[:8])
        elif chunk_type == b'IDAT':
            idat_data.append(chunk_data)
        elif chunk_type == b'IEND':
            break
            
    raw_data = zlib.decompress(b''.join(idat_data))
    row_size = width + 1
    pixels = []
    for y in range(height):
        row = raw_data[y * row_size : (y + 1) * row_size]
        pixels.extend(row[1:])
    return width, height, bytes(pixels)

def main():
    maps_dir = "assets/maps"
    width, height = 180, 180
    layers = ["FULL", "Borders", "Decor", "Objects", "Rocks"]
    
    print("Migrating .byt map layers to grayscale .png format...")
    for layer in layers:
        byt_path = os.path.join(maps_dir, f"BIGisland{layer}.byt")
        png_path = os.path.join(maps_dir, f"BIGisland{layer}.png")
        
        if not os.path.exists(byt_path):
            print(f"Skipping {byt_path}: File not found")
            continue
            
        with open(byt_path, 'rb') as f:
            data = f.read()
            
        if len(data) != width * height:
            print(f"Error: {byt_path} has invalid size {len(data)}")
            continue
            
        write_grayscale_png(png_path, width, height, data)
        print(f"Successfully migrated {byt_path} -> {png_path}")

if __name__ == "__main__":
    main()
