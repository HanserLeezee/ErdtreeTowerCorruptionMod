"""Pack the custom law route icons (art-source/law-icons/*.png, generated originals with transparent backgrounds) into one RGBA atlas.

The law screen and the build panel draw icon N from scene-assets/law-art/icons.rgba: a 5 x 4 grid of 160 x 160 cells, cell N at
(N % 5, N // 5). src/law.rs Node::art holds N; the order below is the art index. Each source is trimmed to its visible pixels and
centred with a small margin, so icons of different framing look the same size. Usage:
python scene-builder/law_icons.py [--source art-source/law-icons] [--output scene-assets/law-art]
"""
import argparse
from pathlib import Path

from PIL import Image

# 序号与 src/law.rs 的 Node::art 对应：强敌、词条蔓延、精英词条、词条强化、坚不可摧、元素庇护、异常耐性、节制、脆弱、孤狼、疲惫、断念、贫瘠、奸商、吝啬的赐福、限时、嫉妒的世界、精英成双
ICONS = ['enemy', 'spread', 'elite', 'empower', 'poise', 'element', 'status', 'temperance', 'fragile', 'lonewolf',
         'fatigue', 'focus', 'poverty', 'merchant', 'stingy', 'timer', 'envy', 'pair']
CELL, COLUMNS, ROWS = 160, 5, 4


def fit(image):
    image = image.convert('RGBA')
    box = image.getchannel('A').point(lambda a: 255 if a > 24 else 0).getbbox() or (0, 0, *image.size)
    image = image.crop(box)
    side = max(image.size)
    square = Image.new('RGBA', (side, side))
    square.paste(image, ((side - image.width) // 2, (side - image.height) // 2))
    inner = CELL - 12
    return square.resize((inner, inner), Image.LANCZOS)


def build(source, output):
    atlas = Image.new('RGBA', (CELL * COLUMNS, CELL * ROWS))
    for index, name in enumerate(ICONS):
        icon = fit(Image.open(source / f'{name}.png'))
        atlas.alpha_composite(icon, ((index % COLUMNS) * CELL + 6, (index // COLUMNS) * CELL + 6))
    output.mkdir(parents=True, exist_ok=True)
    (output / 'icons.rgba').write_bytes(atlas.tobytes())
    preview = Image.new('RGBA', atlas.size, (24, 20, 16, 255)); preview.alpha_composite(atlas)
    return atlas, preview


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', type=Path, default=root / 'art-source/law-icons')
    parser.add_argument('--output', type=Path, default=root / 'scene-assets/law-art')
    parser.add_argument('--preview', type=Path)
    args = parser.parse_args()
    atlas, preview = build(args.source, args.output)
    if args.preview:
        preview.convert('RGB').save(args.preview)
    print(f'Packed {len(ICONS)} law icons into {args.output / "icons.rgba"} ({atlas.width}x{atlas.height})')
