import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class TransmogIntegration(unittest.TestCase):
    def test_release_includes_engine_and_license(self):
        package = (ROOT / 'distribution/package.py').read_text(encoding='utf-8')
        self.assertIn("copy_file(built_native.with_name('tower_transmog.dll'), 'tower_transmog.dll')", package)
        license_text = (ROOT / 'licenses/armor-transmog.txt').read_text(encoding='utf-8')
        self.assertIn('Copyright (c) 2024 Tom Clark', license_text)
        self.assertIn('Permission is hereby granted', license_text)

    def test_engine_does_not_install_gameplay_or_menu_hooks(self):
        root = ROOT / 'native/vendor/tower-transmog'
        cmake = (root / 'CMakeLists.txt').read_text(encoding='utf-8')
        self.assertNotIn('src/ertransmogrify_shop.cpp', cmake)
        self.assertNotIn('src/ertransmogrify_talkscript.cpp', cmake)
        init = (root / 'src/ertransmogrify_vfx.cpp').read_text(encoding='utf-8').split('void vfx::initialize()')[1]
        self.assertNotIn('get_posture_control_detour', init)
        self.assertNotIn('copy_player_character_data_detour', init)
        self.assertNotIn('copy_player_to_blood_message_options_detour', init)

if __name__ == '__main__':
    unittest.main()
