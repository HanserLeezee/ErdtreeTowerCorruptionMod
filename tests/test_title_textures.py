import io
import struct
import sys
import unittest
from pathlib import Path

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scene-builder'))
import gfx_bitmap as gb
import title_textures as tt


class TitleTextures(unittest.TestCase):
    def test_loading_background_loops_below_native_gauge(self):
        header=b'GFX\x0b\0\0\0\0'+gb.rect(0,38400,0,21600)+struct.pack('<HH',30<<8,1)
        tags=[(26,gb.place(1,1,'Black')),(26,gb.place(5,2,'Gauge'))]
        tags += [(26,gb.place(12+i,20+i,'TowerThemeArt'+str(i))) for i in range(3)]
        original=gb.build(header,tags+[(1,b''),(26,b'\x01'+struct.pack('<H',5)),(0,b'')])
        fixed=tt.loading_underlay(original)
        _,result=gb.parse(fixed)
        placements=[b for c,b in result if c==26]
        self.assertEqual([struct.unpack_from('<H',b,1)[0] for b in placements],[1,6,2,6])
        cycle=next(b for c,b in result if c==39)
        self.assertEqual(struct.unpack_from('<H',cycle,2)[0],720)
        self.assertEqual(sum(c==1 for c,b in gb.read_tags(cycle,4,len(cycle))),720)
        self.assertEqual(tt.loading_underlay(fixed),fixed)

    def test_title_background_is_below_native_controls_without_frame_scripts(self):
        inner = [(26, gb.place(2, 10, 'CommandList')), (26, gb.place(13, 11, 'PressStart')),
                 (26, gb.place(21, 12, 'TowerThemeArt', -960, -540)), (1, b''),
                 (26, b'\x01'+struct.pack('<H', 2)), (0, b'')]
        sprite = struct.pack('<HH', 5, 2) + b''.join(gb.tag(c,b) for c,b in inner)
        symbol = struct.pack('<HH', 1, 5) + b'_05_000_Title_fla.Timeline_1\0'
        header = b'GFX\x0b\0\0\0\0' + gb.rect(0,38400,0,21600) + struct.pack('<HH',30<<8,1)
        original = gb.build(header, [(39,sprite),(76,symbol),(1,b''),(0,b'')])
        fixed = tt.title_underlay(original)
        _, tags = gb.parse(fixed)
        body = next(b for c,b in tags if c==39)
        placed = [b for c,b in gb.read_tags(body,4,len(body)) if c==26]
        self.assertEqual([struct.unpack_from('<H',b,1)[0] for b in placed], [3,14,1,3])
        self.assertEqual(placed[0], gb.place(3,10,'CommandList',-550,90))
        self.assertEqual(placed[1], gb.place(14,11,'PressStart',-550,160))
        self.assertEqual(tt.title_underlay(fixed),fixed)

    def test_external_image_matches_native_gfx_encoding(self):
        expected = bytes.fromhex('010000000d00000400020e4d454e555f4974656d5061727473124d454e555f4974656d50617274732e746761')
        self.assertEqual(tt.external_image(1, 'MENU_ItemParts', (1024, 512)), expected)

    def test_conversion_preserves_every_non_image_tag_and_bitmap_id(self):
        picture = io.BytesIO()
        Image.new('RGB', (64, 36), 'red').save(picture, 'JPEG')
        header = b'GFX\x0b\0\0\0\0' + gb.rect(0, 38400, 0, 21600) + struct.pack('<HH', 30 << 8, 1)
        original = gb.build(header, [(21, struct.pack('<H', 42) + picture.getvalue()), (82, b'untouched script'), (1, b''), (0, b'')])
        result = tt.externalize(original, ['title'])
        _, tags = gb.parse(result)
        self.assertEqual(tags[0], (1009, tt.external_image(42, 'MENU_TowerTheme_title', (64, 36))))
        self.assertEqual(tags[1:], gb.parse(original)[1][1:])
        self.assertEqual(tt.externalize(result, ['title']), result)
        with self.assertRaises(ValueError):
            tt.externalize(original, ['title', 'loading1'])


if __name__ == '__main__':
    unittest.main()
