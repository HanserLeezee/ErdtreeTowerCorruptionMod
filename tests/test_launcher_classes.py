import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class LauncherClasses(unittest.TestCase):
    """启动器轮播的职业介绍与游戏里的职业界面（native/src/class_ui.rs 的 ROLES）一致"""

    def test_carousel_details_match_the_in_game_classes(self):
        rust = (ROOT / 'native/src/class_ui.rs').read_text(encoding='utf-8')
        roles = re.findall(r'Role\{name:"([^"]+)",role:"([^"]+)",mechanic:\("([^"]+)","([^"]+)"\),kit:"([^"]+)",bias:"([^"]+)",accent:\((\d+),(\d+),(\d+)\)\}', rust)
        config = (ROOT / 'launcher/ui/js/config.js').read_text(encoding='utf-8')
        details = re.findall(r"detail: \{ name: '([^']+)', role: '([^']+)', mechanic: \['([^']+)', '([^']+)'\], kit: '([^']+)', bias: '([^']+)', accent: 'rgb\((\d+), (\d+), (\d+)\)' \}", config)
        self.assertEqual(len(roles), 6)
        self.assertEqual(details, roles)


if __name__ == '__main__':
    unittest.main()
