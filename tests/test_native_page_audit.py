import importlib.util
from pathlib import Path
import struct
import unittest

spec = importlib.util.spec_from_file_location('audit', Path(__file__).resolve().parents[1] / 'distribution/native_page_audit.py')
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class PageAuditTests(unittest.TestCase):
    def test_ranges_are_sorted_and_gaps_are_not_inferred(self):
        ranges = audit.unwind_ranges(struct.pack('<9I', 30, 40, 2, 10, 20, 1, 0, 0, 0))
        starts = [row[0] for row in ranges]
        self.assertEqual(audit.containing(ranges, starts, 10), (10, 20, 1))
        for address in (0, 20, 25, 40):
            self.assertIsNone(audit.containing(ranges, starts, address))

    def test_truncated_directory_is_rejected(self):
        with self.assertRaises(ValueError):
            audit.unwind_ranges(b'\0' * 13)


if __name__ == '__main__':
    unittest.main()
