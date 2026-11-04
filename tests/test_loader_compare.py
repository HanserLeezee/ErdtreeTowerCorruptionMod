import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('loader_compare', ROOT / 'distribution/loader_compare.py')
compare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(compare)


class LoaderCompare(unittest.TestCase):
    def test_solo_save_and_resources_match(self):
        me3, yaf = compare.configs('F:/Tower/mod.dll', 'F:/Tower/scene-assets', None)
        self.assertIn('ER_TOWER_LOADER_TEST.sl2', me3)
        self.assertIn('replace_save_filename=ER_TOWER_LOADER_TEST.sl2', yaf)
        for text in (me3, yaf):
            self.assertIn('F:/Tower/mod.dll', text)
            self.assertIn('F:/Tower/scene-assets', text)
        self.assertNotIn('ersc.dll', yaf)

    def test_seamless_dependency_and_save_are_explicit(self):
        me3, yaf = compare.configs('F:/Tower/mod.dll', 'F:/Tower/scene-assets', 'F:/Test/SeamlessCoop/ersc.dll')
        self.assertLess(yaf.index('seamless='), yaf.index('tower='))
        self.assertIn('|after,seamless', yaf)
        self.assertIn('load_after = [{ id = "ersc.dll", optional = false }]', me3)
        self.assertIn('replace_seamless_coop_save_filename=', yaf)
        self.assertIn('savefile = "ER_TOWER_LOADER_TEST.sl2"', me3)

    def test_delimiters_are_rejected(self):
        for path in ('F:/bad|early.dll', 'F:/bad\n[dll]', "F:/bad'name.dll"):
            with self.assertRaises(ValueError): compare.configs(path, 'F:/assets', None)
