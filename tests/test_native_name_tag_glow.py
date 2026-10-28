import importlib.util
import re
import sys
import unittest
from pathlib import Path

root = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(root / 'scene-builder'))
spec = importlib.util.spec_from_file_location('name_tag_glow', root / 'scene-builder/native_name_tag_glow.py')
tag = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tag)

SOURCE = '''   public dynamic class EnemyTag_ColorText_14 extends MovieClip
   {

      public var Text_0:MovieClip;

      public function EnemyTag_ColorText_14()
      {
         super();
         addFrameScript(0,this.frame1);
      }

      internal function frame1() : *
      {
         stop();
      }
   }
'''


def rust_titles():
    rust = (root / 'src/titles.rs').read_text(encoding='utf-8')
    order = re.search(r'pub const ALL:\[Style;\d+\]=\[([^\]]*)\]', rust).group(1)
    styles = [v.strip().removeprefix('Style::') for v in order.split(',') if v.strip()]
    names = dict(re.findall(r'Style::(\w+)=>"([^"]+)"', re.search(r'pub fn name\(self\)[^}]*\}', rust, re.S).group(0)))
    colors = {k: (int(a, 16), int(b, 16)) for k, a, b in re.findall(r'Style::(\w+)=>\(0x([0-9A-Fa-f]{6}),0x([0-9A-Fa-f]{6})\)', rust)}
    titles = re.findall(r'Title\{id:(\d+),text:"([^"]+)",style:Style::(\w+)', rust)
    return styles, names, colors, titles


class NameTagGlow(unittest.TestCase):
    def test_styles_and_titles_match_the_rust_tables(self):
        styles, names, colors, titles = rust_titles()
        self.assertEqual([name for name, _, _, _ in tag.STYLES], [names[s] for s in styles])
        self.assertEqual([(c, a) for _, c, a, _ in tag.STYLES], [colors[s] for s in styles])
        self.assertEqual(tag.TITLES, [(text, styles.index(style)) for _, text, style in titles])

    def test_titles_use_only_level_one_gb2312(self):
        for text, _ in tag.TITLES:
            for ch in text:
                high = ch.encode('gb2312')[0]
                self.assertTrue(0xB0 <= high <= 0xD7, f'{text}: {ch} may be missing from the native font')

    def test_patch_only_touches_teammate_names_and_refuses_twice(self):
        patched = tag.patch_source(SOURCE)
        self.assertIn('addEventListener(flash.events.Event.ENTER_FRAME,this.towerTagUpdate);', patched)
        self.assertIn('this.parent.name.indexOf("RemotePCTag") != 0', patched)
        self.assertIn('this.name != "Name"', patched)
        self.assertNotIn('getTimer', patched)
        self.assertNotIn('function(', patched.replace('private function', ''))
        with self.assertRaises(ValueError):
            tag.patch_source(patched)


if __name__ == '__main__':
    unittest.main()
