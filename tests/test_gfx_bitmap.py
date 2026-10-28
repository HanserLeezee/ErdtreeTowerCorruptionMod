import io
import struct
import sys
import unittest
from pathlib import Path

from PIL import Image

root = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(root / 'scene-builder'))
import gfx_bitmap as gb  # noqa: E402


def movie(tags):
    rect = gb.rect(0, 1920 * 20, 0, 1080 * 20)
    head = b'GFX\x0b\0\0\0\0' + rect + struct.pack('<HH', 30 << 8, 1)
    return gb.build(head, tags)


def picture(color=(200, 150, 60)):
    buffer = io.BytesIO(); Image.new('RGB', (64, 36), color).save(buffer, 'JPEG'); return buffer.getvalue()


class Bits:
    def __init__(self, data):
        self.data, self.pos = data, 0

    def unsigned(self, n):
        v = 0
        for _ in range(n):
            v = (v << 1) | ((self.data[self.pos >> 3] >> (7 - (self.pos & 7))) & 1); self.pos += 1
        return v

    def signed(self, n):
        v = self.unsigned(n)
        return v - (1 << n) if n and v >> (n - 1) else v


class GfxBitmap(unittest.TestCase):
    def test_root_embedding_adds_definitions_and_named_placements_only(self):
        base = movie([(9, b'\x00\x00\x00'), (gb.SHOW_FRAME, b''), (gb.END, b'')])
        pictures = [('TowerThemeArt0', picture()), ('TowerThemeArt1', picture((40, 60, 200)))]
        out = gb.embed(base, pictures)
        self.assertEqual(gb.verify(base, out, [p for _, p in pictures]), 9)
        _, tags = gb.parse(out)
        names = [body[body.index(b'Tower'):-1].decode() for code, body in tags if code == gb.PLACE_OBJECT2]
        self.assertEqual(names, ['TowerThemeArt0', 'TowerThemeArt1'])
        self.assertLess(max(i for i, t in enumerate(tags) if t[0] == gb.PLACE_OBJECT2), [c for c, _ in tags].index(gb.SHOW_FRAME))

    def test_shape_is_a_full_screen_clipped_bitmap_rectangle(self):
        body = gb.bitmap_shape(7, 6, 1920, 1080, (1672, 941))
        self.assertEqual(struct.unpack_from('<H', body)[0], 7)
        bits = Bits(body[2:]); n = bits.unsigned(5)
        self.assertEqual([bits.signed(n) for _ in range(4)], [0, 1920 * 20, 0, 1080 * 20])
        rest = body[2 + (bits.pos + 7) // 8:]
        self.assertEqual((rest[0], rest[1], struct.unpack_from('<H', rest, 2)[0]), (1, 0x41, 6))
        m = Bits(rest[4:]); self.assertEqual(m.unsigned(1), 1); k = m.unsigned(5)
        self.assertAlmostEqual(m.signed(k) / 65536, 20 * 1920 / 1672, places=3)

    def test_sprite_embedding_keeps_other_tags_and_rejects_tampering(self):
        sprite = struct.pack('<HH', 5, 1) + gb.tag(gb.SHOW_FRAME, b'') + gb.tag(gb.END, b'')
        symbol = struct.pack('<HH', 1, 5) + b'Pkg.Timeline_1\0'
        base = movie([(gb.DEFINE_SPRITE, sprite), (gb.SYMBOL_CLASS, symbol), (gb.SHOW_FRAME, b''), (gb.END, b'')])
        pictures = [('TowerThemeArt', picture())]
        out = gb.embed(base, pictures, 'Pkg.Timeline_1', (-960, -540))
        gb.verify(base, out, [p for _, p in pictures], 'Pkg.Timeline_1')
        head, tags = gb.parse(out)
        broken = gb.build(head, [t for t in tags if t[0] != gb.SYMBOL_CLASS] + [(gb.SYMBOL_CLASS, symbol[:-1] + b'X\0')])
        with self.assertRaises(ValueError):
            gb.verify(base, broken, [p for _, p in pictures], 'Pkg.Timeline_1')


if __name__ == '__main__':
    unittest.main()
