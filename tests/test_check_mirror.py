import importlib.util
import shutil
import tempfile
import threading
import unittest
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

DIST = Path(__file__).resolve().parents[1] / 'distribution'


def load(name):
    spec = importlib.util.spec_from_file_location(name, DIST / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


pack = load('release_pack')
mirror = load('check_mirror')


class RangeHandler(SimpleHTTPRequestHandler):
    """Static files with single-range support, like COS / GitHub downloads."""
    ranges = True

    def log_message(self, *args):
        pass

    def do_GET(self):
        path = Path(self.translate_path(self.path))
        header = self.headers.get('Range')
        if not self.ranges or not header or not path.is_file():
            return super().do_GET()
        first, last = (int(v) for v in header.removeprefix('bytes=').split('-'))
        data = path.read_bytes()[first:last + 1]
        self.send_response(206)
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)


class NoRangeHandler(RangeHandler):
    ranges = False


class CheckMirror(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        stage = self.root / 'stage'
        (stage / 'scene-assets').mkdir(parents=True)
        (stage / 'erdtree_tower.dll').write_bytes(b'dll' * 5000)
        (stage / 'scene-assets/regulation.bin').write_bytes(bytes(range(256)) * 40)
        out = self.root / 'out'
        pack.build(stage, 3, out)
        # The layout package.py writes to dist/mirror-v<N>/ and the user uploads to the bucket
        releases = self.root / 'site/releases'
        (releases / 'download/v3').mkdir(parents=True)
        (releases / 'latest/download').mkdir(parents=True)
        shutil.copy(out / 'tower-pack-v3.bin', releases / 'download/v3')
        shutil.copy(out / 'tower-release.json', releases / 'download/v3')
        shutil.copy(out / 'tower-release.json', releases / 'latest/download')

    def tearDown(self):
        shutil.rmtree(self.root)

    def serve(self, handler):
        server = ThreadingHTTPServer(('127.0.0.1', 0), partial(handler, directory=str(self.root / 'site')))
        threading.Thread(target=server.serve_forever, daemon=True).start()
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        return f'http://127.0.0.1:{server.server_address[1]}/releases'

    def test_a_correct_mirror_passes(self):
        manifest, problems = mirror.check(self.serve(RangeHandler), expect=3)
        self.assertEqual(manifest['version'], 3)
        self.assertEqual(problems, [])

    def test_missing_range_support_stale_latest_and_missing_pack_are_reported(self):
        _, problems = mirror.check(self.serve(NoRangeHandler))
        self.assertTrue(any('不支持分段下载' in p for p in problems))
        base = self.serve(RangeHandler)
        _, problems = mirror.check(base, expect=4)
        self.assertTrue(any('不是 v4' in p for p in problems))
        (self.root / 'site/releases/download/v3/tower-pack-v3.bin').unlink()
        _, problems = mirror.check(base)
        self.assertTrue(any('HTTP 404' in p for p in problems))

    def test_unreadable_manifest_is_reported(self):
        (self.root / 'site/releases/latest/download/tower-release.json').unlink()
        manifest, problems = mirror.check(self.serve(RangeHandler))
        self.assertIsNone(manifest)
        self.assertIn('HTTP 404', problems[0])


if __name__ == '__main__':
    unittest.main()
