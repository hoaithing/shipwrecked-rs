import os
import zlib
import struct
import random
import math

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

def noise(x, y):
    return (math.sin(x * 0.08) + math.cos(y * 0.08) + 
            math.sin(x * 0.04 + y * 0.03) * 2.0 + 
            math.sin(x * 0.15) * 0.5)

def main():
    maps_dir = "assets/maps"
    old_w, old_h = 180, 180
    new_w, new_h = 540, 540
    
    layers = ["FULL", "Borders", "Decor", "Objects", "Rocks"]
    orig_data = {}
    
    # 1. Load original maps
    for layer in layers:
        png_path = os.path.join(maps_dir, f"BIGisland{layer}.png")
        if not os.path.exists(png_path):
            print(f"Error: {png_path} not found. Please compile/migrate first.")
            return
        w, h, data = read_grayscale_png(png_path)
        if w != old_w or h != old_h:
            print(f"Error: {png_path} has size {w}x{h}, expected {old_w}x{old_h}")
            return
        orig_data[layer] = data

    # 2. Define procedural islands
    # Setup deterministic random seed so map is reproducible if regenerated
    random.seed(12345)
    islands = []
    
    # Generate centers for procedural islands outside the 180x180 top-left area
    # Top-right area (x >= 180, y < 180)
    for _ in range(8):
        cx = random.randint(220, new_w - 40)
        cy = random.randint(40, 140)
        r = random.randint(25, 55)
        islands.append((cx, cy, r))
    # Bottom-left area (x < 180, y >= 180)
    for _ in range(8):
        cx = random.randint(40, 140)
        cy = random.randint(220, new_h - 40)
        r = random.randint(25, 55)
        islands.append((cx, cy, r))
    # Bottom-right area (x >= 180, y >= 180)
    for _ in range(20):
        cx = random.randint(200, new_w - 50)
        cy = random.randint(200, new_h - 50)
        r = random.randint(30, 70)
        islands.append((cx, cy, r))

    # 3. Create expanded layers
    new_layers = {layer: bytearray(new_w * new_h) for layer in layers}
    
    print(f"Generating expanded map ({new_w}x{new_h}) with procedural islands...")
    
    for y in range(new_h):
        for x in range(new_w):
            idx = y * new_w + x
            
            # If in top-left 180x180, copy original J2ME map pixels
            if x < old_w and y < old_h:
                old_idx = y * old_w + x
                for layer in layers:
                    new_layers[layer][idx] = orig_data[layer][old_idx]
                continue
                
            # Otherwise, procedurally generate
            # Compute land factor
            land_factor = -1.0
            for cx, cy, r in islands:
                dist = math.sqrt((x - cx)**2 + (y - cy)**2)
                r_noisy = r + noise(x, y) * 6.0
                if dist < r_noisy:
                    factor = 1.0 - (dist / r_noisy)
                    if factor > land_factor:
                        land_factor = factor
            
            # Select terrain (FULL layer)
            if land_factor > 0:
                if land_factor < 0.15:
                    terrain = 0 # Sand (yellow beach)
                elif land_factor < 0.5:
                    terrain = 1 # Grass (bright green)
                elif land_factor < 0.75:
                    terrain = 2 # Clay/Dirt (clay)
                else:
                    terrain = 3 # Forest (dark green)
            else:
                terrain = 14 # Deep Water (blocks)
                
            new_layers["FULL"][idx] = terrain
            new_layers["Borders"][idx] = 0 # No borders in expanded region
            
            # Populate Decor, Rocks, and Objects based on terrain
            if terrain == 0: # Sand
                # Rocks
                if random.random() < 0.03:
                    new_layers["Rocks"][idx] = random.randint(1, 5) # small beach pebbles
                # Objects
                if random.random() < 0.02:
                    new_layers["Objects"][idx] = random.choice([2, 65, 66, 67, 68]) # Coconuts or shells
            elif terrain == 1: # Grass
                # Decor (Palm trees)
                if random.random() < 0.03:
                    new_layers["Decor"][idx] = random.choice([18, 1, 5])
                # Small grass decor
                elif random.random() < 0.04:
                    new_layers["Decor"][idx] = random.choice([28, 30, 31, 32, 33])
                # Objects (Pickups)
                if random.random() < 0.015:
                    new_layers["Objects"][idx] = random.choice([8, 9, 29]) # Grass/branch/berries
            elif terrain == 2: # Clay
                # Decor (palm trees)
                if random.random() < 0.02:
                    new_layers["Decor"][idx] = random.choice([18, 1, 5])
                # Rocks
                if random.random() < 0.02:
                    new_layers["Rocks"][idx] = random.randint(5, 10)
                # Objects (Stones / Logs)
                if random.random() < 0.015:
                    new_layers["Objects"][idx] = random.choice([4, 12]) # Stones or wood logs
            elif terrain == 3: # Forest
                # Dense palm trees
                if random.random() < 0.07:
                    new_layers["Decor"][idx] = random.choice([18, 1, 5, 13])
                # Shrub decor
                elif random.random() < 0.05:
                    new_layers["Decor"][idx] = random.choice([33, 34, 35, 36])
                # Objects (Wood logs/branches)
                if random.random() < 0.025:
                    new_layers["Objects"][idx] = random.choice([9, 12]) # Branches or logs
                    
    # Write files back
    for layer in layers:
        png_path = os.path.join(maps_dir, f"BIGisland{layer}.png")
        write_grayscale_png(png_path, new_w, new_h, bytes(new_layers[layer]))
        print(f"Expanded map layer saved: {png_path}")
        
    print("Map expansion completed successfully!")

if __name__ == "__main__":
    main()
