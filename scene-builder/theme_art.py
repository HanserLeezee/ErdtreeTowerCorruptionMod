"""Convert the theme-act illustrations (art-source/theme/<art>.jpg, generated originals) into the textures shown on the act title
card: scene-assets/theme-art/<art>.rgba, 800 x 450 RGBA. The art names are src/themes.rs Theme::art. Usage:
python scene-builder/theme_art.py [--source art-source/theme] [--output scene-assets/theme-art]
"""
import argparse
import re
from pathlib import Path

from PIL import Image

SIZE = (800, 450)


def art_names(root):
    rust = (root / 'src/themes.rs').read_text(encoding='utf-8')
    return re.findall(r't\(\d+,\d+,"[^"]+","[^"]+","[^"]+","([a-z0-9]+)"', rust)


def build(root, source, output):
    output.mkdir(parents=True, exist_ok=True)
    names = art_names(root)
    for name in names:
        image = Image.open(source / f'{name}.jpg').convert('RGB')
        # 裁成 16:9 再缩小，避免不同尺寸的源图被拉伸
        w, h = image.size; target = w / h
        if abs(target - 16 / 9) > 0.01:
            nw = min(w, round(h * 16 / 9)); nh = min(h, round(w * 9 / 16))
            image = image.crop(((w - nw) // 2, (h - nh) // 2, (w - nw) // 2 + nw, (h - nh) // 2 + nh))
        (output / f'{name}.rgba').write_bytes(image.resize(SIZE, Image.LANCZOS).convert('RGBA').tobytes())
    return names


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', type=Path, default=root / 'art-source/theme')
    parser.add_argument('--output', type=Path, default=root / 'scene-assets/theme-art')
    args = parser.parse_args()
    names = build(root, args.source, args.output)
    print(f'Converted {len(names)} theme illustrations into {args.output}')
