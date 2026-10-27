import importlib.util
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

DIST = Path(__file__).resolve().parents[1] / 'distribution'
sys.path.insert(0, str(DIST))
spec = importlib.util.spec_from_file_location('upload_mirror', DIST / 'upload_mirror.py')
upload_mirror = importlib.util.module_from_spec(spec)
spec.loader.exec_module(upload_mirror)


class UploadMirror(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        releases = self.root / 'mirror-v7/releases'
        for rel, text in [('download/v7/tower-pack-v7.bin', 'pack'), ('download/v7/tower-release.json', '{"version": 7}'),
                          ('latest/download/tower-release.json', '{"version": 7}')]:
            (releases / rel).parent.mkdir(parents=True, exist_ok=True)
            (releases / rel).write_text(text, encoding='utf-8')

    def tearDown(self):
        shutil.rmtree(self.root)

    def test_target_comes_from_the_default_oss_domain(self):
        self.assertEqual(upload_mirror.target_from_url('https://eldenring-mods-hz.oss-cn-hangzhou.aliyuncs.com/releases'),
                         'oss://eldenring-mods-hz/releases')
        for bad in ['http://eldenring-mods-hz.oss-cn-hangzhou.aliyuncs.com/releases', 'https://mods.example.com/releases']:
            with self.assertRaises(SystemExit):
                upload_mirror.target_from_url(bad)

    def test_packs_go_up_before_any_manifest_and_latest_goes_last(self):
        calls = []
        upload_mirror.upload(self.root / 'mirror-v7', 'oss://bucket/releases', 'ossutil', run=lambda argv, check: calls.append(argv))
        self.assertEqual([c[3] for c in calls], [
            'oss://bucket/releases/download/v7/tower-pack-v7.bin',
            'oss://bucket/releases/download/v7/tower-release.json',
            'oss://bucket/releases/latest/download/tower-release.json',
        ])
        self.assertTrue(all(c[0] == 'ossutil' and c[1] == 'cp' and c[-1] == '-f' for c in calls))

    def test_the_shipped_mirror_url_maps_to_a_bucket(self):
        url = upload_mirror.mirror_url()
        if url:
            self.assertTrue(upload_mirror.target_from_url(url).startswith('oss://'))


if __name__ == '__main__':
    unittest.main()
