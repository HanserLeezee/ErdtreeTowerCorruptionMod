import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('probe', Path(__file__).resolve().parents[1] / 'distribution/menu_lifecycle_probe.py')
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class MenuProbeTests(unittest.TestCase):
    def test_created_and_visible_are_independent(self):
        data = bytearray(0x46)
        data[2], data[3], data[6] = 1, 2, 3
        self.assertEqual(probe.active_states(data), [
            dict(index=2, created=True, visible=False, raw=1),
            dict(index=3, created=False, visible=True, raw=2),
            dict(index=6, created=True, visible=True, raw=3)])

    def test_short_reads_are_rejected(self):
        with self.assertRaises(ValueError):
            probe.active_states(b'\0' * 0x45)


if __name__ == '__main__':
    unittest.main()
