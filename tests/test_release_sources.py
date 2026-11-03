import hashlib
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('release_sources', Path(__file__).resolve().parents[1] / 'distribution/verify_release_sources.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ReleaseSourcesTests(unittest.TestCase):
    def test_only_declared_exact_deviations_and_line_endings_are_accepted(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            assets = root / 'scene-assets'
            assets.mkdir()
            (assets / 'arenas-resources.txt').write_text('')
            (assets / 'sample.txt').write_bytes(b'hello\r\n')
            entry = {'path': 'scene-assets/sample.txt', 'size': 6, 'sha256': hashlib.sha256(b'hello\n').hexdigest(), 'lfSha256': hashlib.sha256(b'hello\n').hexdigest()}
            self.assertEqual(module.audit(root, {'files': [entry]}, [])['newlineOnly'], 1)
            (assets / 'sample.txt').write_bytes(b'changed')
            with self.assertRaises(ValueError):
                module.audit(root, {'files': [entry]}, [])
            deviation = {'path': entry['path'], 'sha256': hashlib.sha256(b'changed').hexdigest()}
            self.assertEqual(module.audit(root, {'files': [entry]}, [deviation])['documentedDifferences'], [entry['path']])

    def test_uncommitted_generated_data_blocks_release(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                return subprocess.run(['git', *args], cwd=root, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            git('init')
            git('-c', 'user.name=Source Test', '-c', 'user.email=test@example.invalid', 'commit', '--allow-empty', '-m', 'baseline')
            module.require_committed(root)
            (root / 'scene-assets').mkdir()
            (root / 'scene-assets/boss-pool.txt').write_text('generated')
            with self.assertRaises(ValueError):
                module.require_committed(root)
            git('add', 'scene-assets/boss-pool.txt')
            with self.assertRaises(ValueError):
                module.require_committed(root)


if __name__ == '__main__':
    unittest.main()
