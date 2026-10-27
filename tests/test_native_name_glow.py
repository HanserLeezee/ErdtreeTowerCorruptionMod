import importlib.util
import struct
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('name_glow', Path(__file__).resolve().parents[1] / 'scene-builder/native_name_glow.py')
glow = importlib.util.module_from_spec(spec)
spec.loader.exec_module(glow)

def gfx(code, image=b'original'):
    body = b'\x08\x00\x00\x1e\x01\x00'
    for tag, data in [(21, image), (82, code), (0, b'')]:
        body += struct.pack('<H', tag << 6 | len(data)) + data
    return b'GFX\x0b' + struct.pack('<I', len(body) + 8) + body

class NameGlowSafety(unittest.TestCase):
    def test_prefix_matcher_does_not_depend_on_gfx_unicode_regex(self):
        source = '      public function MainTimeline() { addFrameScript(0,this.frame1,10,this.frame11,18,this.frame19); }'
        patched = glow.patch_source(source, 'Status_Weapon1')
        self.assertIn('this.towerTier(field.text)', patched)
        self.assertIn('charCodeAt', patched)
        self.assertNotIn('.test(label)', patched)
        self.assertNotIn('replace(/', patched)

    def test_only_script_change_is_allowed(self):
        self.assertEqual(glow.verify_tags(gfx(b'old'), gfx(b'new code')), 3)

    def test_unrelated_assets_must_not_change(self):
        with self.assertRaises(ValueError):
            glow.verify_tags(gfx(b'old'), gfx(b'new', b'changed image'))

    def test_movie_timing_must_not_change(self):
        altered = bytearray(gfx(b'new'))
        altered[11] = 60
        with self.assertRaises(ValueError):
            glow.verify_tags(gfx(b'old'), bytes(altered))

    def test_noop_and_non_gfx_are_rejected(self):
        with self.assertRaises(ValueError):
            glow.verify_tags(gfx(b'old'), gfx(b'old'))
        with self.assertRaises(ValueError):
            glow.tags(b'FWS' + gfx(b'old')[3:])

    def test_duplicate_or_unknown_timeline_is_rejected(self):
        source = '      public function MainTimeline() { addFrameScript(0,this.frame1,10,this.frame11,18,this.frame19); }'
        patched = glow.patch_source(source, 'Status_Armor')
        with self.assertRaises(ValueError):
            glow.patch_source(patched, 'Status_Armor')
        with self.assertRaises(ValueError):
            glow.patch_source('unexpected source', 'Status_Armor')

class NameGlowRebuild(unittest.TestCase):
    DECOMPILED = """   public dynamic class MainTimeline extends MovieClip
   {
      
      public var DetailStatusView:MovieClip;
      
      private var towerName:TextField;
      
      private var towerSparkles:Sprite;
      
      public function MainTimeline()
      {
         super();
         addFrameScript(0,this.frame1,10,this.frame11,18,this.frame19);
         addEventListener(Event.ENTER_FRAME,this.towerUpdateName);
      }
      
      private function towerDecorate(field:TextField, phase:Number) : void
      {
         if(phase > 0)
         {
            field.text = "";
         }
      }
      
      private function towerUpdateName(event:Event) : void
      {
      }
      
      internal function frame1() : *
      {
         stop();
      }
   }
"""

    def test_previous_build_is_stripped_and_repatched(self):
        stripped = glow.strip_previous(self.DECOMPILED)
        self.assertNotIn('tower', stripped)
        self.assertIn('internal function frame1()', stripped)
        self.assertIn('public var DetailStatusView:MovieClip;', stripped)
        patched = glow.patch_source(stripped, 'Status_Weapon1', 'gold')
        self.assertEqual(patched.count('addEventListener(flash.events.Event.ENTER_FRAME,this.towerUpdateName);'), 1)
        self.assertIn('towerStyle:String = "gold"', patched)

    def test_style_must_be_known(self):
        source = '      public function MainTimeline() { addFrameScript(0,this.frame1,10,this.frame11,18,this.frame19); }'
        with self.assertRaises(ValueError):
            glow.patch_source(source, 'Status_Armor', 'rainbow')
        self.assertIn('towerStyle:String = "auric"', glow.patch_source(source, 'Status_Armor'))

    def test_every_quality_prefix_is_matched_by_code_point(self):
        for word, tier in [('传说', 3), ('史诗', 2), ('稀有', 1), ('普通', 0)]:
            first, second = (ord(c) for c in word)
            self.assertIn(f'first == {first} && second == {second} ? {tier}', glow.MEMBERS.replace('(', '').replace(')', ''))

    def test_effect_never_shadows_the_flash_package(self):
        self.assertNotRegex(glow.MEMBERS, r'var flash\b')

if __name__ == '__main__':
    unittest.main()
