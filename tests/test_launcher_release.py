from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'distribution'))
from release_pack import build
from package_launcher import build_update


class LauncherReleaseTests(unittest.TestCase):
    def test_launcher_only_release_preserves_mod_and_reuses_blobs(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            stage = root / 'old'
            stage.mkdir()
            (stage / 'mod.dll').write_bytes(b'unchanged game content')
            (stage / 'ErdtreeTowerLauncher.exe').write_bytes(b'old launcher')
            old = build(stage, 96, root / 'v96', display_version='1.0.0')
            fresh = root / 'launcher'
            fresh.mkdir()
            (fresh / 'ErdtreeTowerLauncher.exe').write_bytes(b'new launcher')
            result = build_update(fresh, root / 'v96/tower-release.json', 97, '1.1.1', root / 'v97')
            old_mod = next(f for f in old['files'] if f['path'] == 'mod.dll')
            self.assertIn(old_mod, result['files'])
            self.assertEqual(old['blobs'][old_mod['sha256']], result['blobs'][old_mod['sha256']])
            self.assertEqual('1.0.0', result['displayVersion'])
            self.assertEqual('1.1.1', result['launcherVersion'])
            self.assertEqual('launcher', result['updateKind'])
            self.assertEqual(97, result['version'])

    def test_rejects_non_launcher_payload_and_reused_version(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            stage = root / 'stage'
            stage.mkdir()
            (stage / 'mod.dll').write_bytes(b'game')
            build(stage, 96, root / 'old')
            with self.assertRaises(ValueError):
                build_update(stage, root / 'old/tower-release.json', 97, '1.1.1', root / 'bad')
            (stage / 'mod.dll').unlink()
            (stage / 'ErdtreeTowerLauncher.exe').write_bytes(b'launcher')
            with self.assertRaises(ValueError):
                build_update(stage, root / 'old/tower-release.json', 96, '1.1.1', root / 'bad2')


if __name__ == '__main__':
    unittest.main()
