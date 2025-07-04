# https://www.spritefusion.com/editor

"""
Sprite Tile Combination Generator

This script processes a JSON sprite sheet file to generate combinations of tiles that can be
placed adjacent to each other based on their edge compatibility.

What it does:
1. Reads a JSON file containing sprite tile data with a tile layout
2. Extracts tile information including IDs, positions, and sprite sheet properties
3. Analyzes predefined subtile patterns (represented as 4-character strings like "ssgg")
   where each character represents an edge: top, right, bottom, left
   - 's' = sand edge
   - 'g' = grass edge
4. Finds all valid tile combinations where:
   - Horizontal combinations: right edge of tile A matches left edge of tile B
   - Vertical combinations: bottom edge of tile A matches top edge of tile B
5. Generates a new sprite layout with all valid combinations positioned on a grid
6. Outputs:
   - Updated JSON file with the combination layout
   - Separate JSON file with the combination data
   - Console output showing found combinations

Usage: python make_combos.py <input.json> <output.json>

The script is designed for use with sprite editing tools like SpriteFusion to automatically
generate tile combination sheets for game development or tile-based graphics.
"""

import sys
import json

# Read the JSON file
with open("src.json", 'r') as f:
    data = json.load(f)


tiles = data["layers"][0]["tiles"]

# Src data is expected to have the beginning of each row, and the last tile, of
# the region.
extents = [int(t["id"]) for t in tiles]
extents.sort()

row_starts = extents[0:-1]
last_row_end = extents[-1]

width = last_row_end - row_starts[-1] + 1
height = len(row_starts)

tile_size = int(data["tileSize"])

ss = tiles[0]["spriteSheetId"]
sx = tiles[0]["scaleX"]


def tile(id, x, y):
    return {"id": str(id), "x": x * 32, "y": y * 32, "spriteSheetId": ss, "scaleX": sx}


subtiles = [
    "ssgg", "ggss", "gggs", "ggsg", "gsgs", "sgsg", "gggg",
    "ssgg", "ggss", "gsgg", "sggg", "gsgs", "sgsg", "gggg",

    "ssgg", "ggss", "gggs", "ggsg", "gsgs", "sgsg", "gggg",
    "ssgg", "ggss", "gsgg", "sggg", "gsgs", "sgsg", "gggg",

    "ssgg", "ggss", "gggs", "ggsg", "gsgs", "sgsg", "gggg",
    "ssgg", "ggss", "gsgg", "sggg", "gsgs", "sgsg", "gggg",

    "sssg", "ssgs", "sssg", "ssgs", "sssg", "ssgs", "gggg",
    "sgss", "gsss", "sgss", "gsss", "sgss", "gsss", "gggg",

    "sggs", "gssg", "sggs", "gssg", "sggs", "gssg", "gggg",
    "gssg", "sggs", "gssg", "sggs", "gssg", "sggs", "gggg"
]


tiles = []

ids = [x + rs for rs in row_starts for x in range(0, width)]

indexes = dict([(id, index) for (index, id) in enumerate(ids)])


assert (len(ids) == len(subtiles))


combos = [(i, j) for i in ids for j in ids]

x = 0
y = 0

max_x = 256

done = set()

horizontal_combos = []
vertical_combos = []

for (a, b) in combos:
    if done.__contains__((b, a)):
        continue
    done.add((a, b))

    # layouts for a & b
    la = subtiles[indexes[a]]
    lb = subtiles[indexes[b]]

    # right edge of a
    a_r = la[1] + la[3]

    # left edge of b
    b_l = lb[0] + lb[2]

    if a_r == b_l:
        horizontal_combos.append((a, b))

    # bottom edge of a
    a_r = la[2] + la[3]

    # top edge of b
    b_t = lb[0] + lb[1]

    if a_r == b_t:
        vertical_combos.append((a, b))

for (a, b) in horizontal_combos:
    tiles.append(tile(a, x, y))
    tiles.append(tile(b, x + 1, y))
    x = x + 3
    if x > max_x:
        y = y + 3
        x = 0

for (a, b) in vertical_combos:
    tiles.append(tile(a, x, y))
    tiles.append(tile(b, x, y + 1))
    x = x + 3
    if x > max_x:
        y = y + 3
        x = 0


print(horizontal_combos)
print(vertical_combos)


data["layers"][0]["tiles"] = tiles

with open("dst.json", 'w') as f:
    json.dump(data, f, indent=2)


c = {"horizontal": horizontal_combos, "vertical": vertical_combos}
with open("combos.json", 'w') as f:
    json.dump(c, f, indent=2)
