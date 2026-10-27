import importlib.util
import re
import sys
import unittest
from pathlib import Path

root = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(root / 'scene-builder'))
spec = importlib.util.spec_from_file_location('boss_bar_glow', root / 'scene-builder/native_boss_bar_glow.py')
bar = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bar)

SOURCE = '''   public dynamic class BossFE_Bar_Ani_40 extends MovieClip
   {

      public function BossFE_Bar_Ani_40()
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


def affix_order():
    rust = (root / 'src/boss_affixes.rs').read_text(encoding='utf-8')
    order = re.search(r'pub const ALL:\[Affix;\d+\]=\[([^\]]*)\]', rust).group(1)
    variants = [v.strip().removeprefix('Affix::') for v in order.split(',') if v.strip()]
    names = dict(re.findall(r'Affix::(\w+)=>"([^"]+)"', re.search(r'pub fn name\(self\)[^}]*\}', rust, re.S).group(0)))
    return variants, names


class BossBarGlow(unittest.TestCase):
    def test_affix_words_match_the_rules_in_order(self):
        variants, names = affix_order()
        self.assertEqual([name for name, _, _ in bar.AFFIXES], [names[v] for v in variants])

    def test_affix_colours_match_the_overlay(self):
        variants, _ = affix_order()
        style = (root / 'native/src/fancy_text.rs').read_text(encoding='utf-8')
        body = style[style.index('pub fn affix_style'):]
        colours = {v: tuple(int(x) for x in c.split(',')) for v, c in re.findall(r'Affix::(\w+)=>\(\[(\d+,\d+,\d+)\]', body)}
        for variant, (_, colour, _) in zip(variants, bar.AFFIXES):
            r, g, b = colours[variant]
            self.assertEqual(colour, (r << 16) | (g << 8) | b, variant)

    def test_source_is_patched_once_and_only_with_ascii_literals(self):
        patched = bar.patch_source(SOURCE)
        self.assertEqual(patched.count('addEventListener(flash.events.Event.ENTER_FRAME,this.towerBarUpdate);'), 1)
        names_line = next(line for line in patched.splitlines() if 'towerAffixNames:Array' in line)
        self.assertTrue(names_line.isascii())
        with self.assertRaises(ValueError):
            bar.patch_source(patched)
        with self.assertRaises(ValueError):
            bar.patch_source('unexpected class')

    def test_escape_round_trips(self):
        for name, _, _ in bar.AFFIXES:
            literal = bar.escape(name)
            self.assertTrue(literal.isascii())
            self.assertEqual(literal[1:-1].encode('ascii').decode('unicode_escape'), name)


if __name__ == '__main__':
    unittest.main()
