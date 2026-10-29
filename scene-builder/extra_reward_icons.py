"""特殊奖励（武器强化、圣杯扩容、记忆扩容、卢恩）的卡片图标：把生成的图集切成 160×160 图标。

用法：python scene-builder/extra_reward_icons.py scene-assets
读取 scene-assets/reward-art/extra-icons.png（2 列 × 2 行，透明背景，提示词见同目录 extra-icons.txt）
与 stone-icon.png（单个，强化石），写出 scene-assets/reward-icons/990100.rgba … 990104.rgba，
顺序与 native/src/reward_ui.rs 的 EXTRA_ICONS 一致。
"""
from pathlib import Path
import sys
from PIL import Image

FIRST, SIZE, COLUMNS, ROWS = 990100, 160, 2, 2
NAMES = ["武器强化", "圣杯扩容", "记忆扩容", "卢恩"]

assets = Path(sys.argv[1])


def write(tile, index, name):
    bounds = tile.getchannel("A").point(lambda a: 255 if a > 16 else 0).getbbox()
    if bounds is None:
        sys.exit(f"{name} 是空的")
    tile = tile.crop(bounds)
    side = max(tile.size)
    square = Image.new("RGBA", (side, side))
    square.paste(tile, ((side - tile.width) // 2, (side - tile.height) // 2))
    canvas = Image.new("RGBA", (SIZE, SIZE))
    canvas.paste(square.resize((SIZE - 16, SIZE - 16), Image.Resampling.LANCZOS), (8, 8))
    (assets / "reward-icons" / f"{FIRST + index}.rgba").write_bytes(canvas.tobytes())


with Image.open(assets / "reward-art" / "extra-icons.png") as atlas:
    atlas = atlas.convert("RGBA")
    if atlas.getextrema()[3][0] == 255:
        sys.exit("extra-icons.png 没有透明背景")
    tile_w, tile_h = atlas.width / COLUMNS, atlas.height / ROWS
    for index, name in enumerate(NAMES):
        col, row = index % COLUMNS, index // COLUMNS
        write(atlas.crop((round(col * tile_w), round(row * tile_h), round((col + 1) * tile_w), round((row + 1) * tile_h))), index, name)
with Image.open(assets / "reward-art" / "stone-icon.png") as stone:
    write(stone.convert("RGBA"), len(NAMES), "强化石")
print(f"Wrote {len(NAMES) + 1} extra reward icons ({FIRST}-{FIRST + len(NAMES)})")
