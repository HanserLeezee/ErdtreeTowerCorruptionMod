import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('resource_filter', ROOT / 'distribution/resource_filter.py')
resources = importlib.util.module_from_spec(spec)
spec.loader.exec_module(resources)


class ResourceFilter(unittest.TestCase):
    def test_unknown_and_runtime_resources_remain(self):
        for name in ('theme-art/loading1.rgba', 'class-art/classes.rgba', 'law-art/icons.rgba',
                     'law-art/icons-line.rgba', 'fonts/ui-sans-glyphs.txt', 'fonts/ui-glyphs.txt',
                     'reward-details.tsv', 'enemy-pools.txt', 'battlefields.txt',
                     'menu/hi/01_common.tpf.dcx', 'event/m11_10_00_00.emevd.dcx', 'new-data.tsv'):
            self.assertTrue(resources.include_scene_asset(name), name)

    def test_audited_files_stay_in_source_but_not_package(self):
        for name in resources.EXCLUSIONS:
            self.assertTrue((ROOT / 'scene-assets' / name).is_file(), name)
            self.assertFalse(resources.include_scene_asset(name), name)

    def test_embedded_tables_have_only_compile_time_runtime_references(self):
        rust = list((ROOT / 'src').rglob('*.rs')) + list((ROOT / 'native/src').rglob('*.rs'))
        lines = [line for p in rust for line in p.read_text(encoding='utf-8-sig').splitlines()]
        scripts = list(ROOT.glob('*.ps1')) + list((ROOT / 'distribution').glob('*.ps1'))
        for name in resources.EMBEDDED:
            references = [line for line in lines if name in line]
            self.assertTrue(references, name)
            self.assertTrue(all('include_str!' in line for line in references), name)
            for script in scripts:
                self.assertNotIn(name, script.read_text(encoding='utf-8-sig'), str(script))

    def test_build_only_files_have_no_runtime_code_references(self):
        for folder in ('src', 'native/src', 'launcher/Tower.Launcher', 'launcher/Tower.Update'):
            for path in (ROOT / folder).rglob('*'):
                if path.suffix not in ('.cs', '.rs') or 'obj' in path.parts or 'bin' in path.parts:
                    continue
                text = path.read_text(encoding='utf-8-sig')
                for name in resources.BUILD_ONLY:
                    self.assertNotIn(Path(name).name, text, str(path))
