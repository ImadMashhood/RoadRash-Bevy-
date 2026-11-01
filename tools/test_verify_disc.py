"""Authored ISO fixtures. No original game bytes."""
import tempfile
import unittest
from pathlib import Path
import verify_disc


def record(name, sector, length, directory=False):
    raw = bytearray(34 + len(name))
    raw[0] = len(raw)
    raw[2:6] = sector.to_bytes(4, 'little')
    raw[6:10] = sector.to_bytes(4, 'big')
    raw[10:14] = length.to_bytes(4, 'little')
    raw[14:18] = length.to_bytes(4, 'big')
    raw[25] = 2 if directory else 0
    raw[32] = len(name)
    raw[33:33 + len(name)] = name
    return raw


def image(name=b'TEST.BIN;1', sector=21, length=3):
    data = bytearray(22 * 2048)
    data[16 * 2048:16 * 2048 + 7] = b'\x01CD001\x01'
    root = record(b'\0', 20, 2048, True)
    data[16 * 2048 + 156:16 * 2048 + 156 + len(root)] = root
    entry = record(name, sector, length)
    data[20 * 2048:20 * 2048 + len(entry)] = entry
    data[21 * 2048:21 * 2048 + 3] = b'abc'
    return data


class InventoryTests(unittest.TestCase):
    def parse(self, data):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'fixture.iso'
            path.write_bytes(data)
            return verify_disc.inventory(path)

    def test_authored_file_hash(self):
        result = self.parse(image())
        self.assertEqual(result['/TEST.BIN']['size'], 3)
        self.assertEqual(result['/TEST.BIN']['sha256'], 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad')

    def test_rejects_bad_signature_and_short_image(self):
        for data in (b'', bytearray(22 * 2048)):
            with self.assertRaises(ValueError):
                self.parse(data)

    def test_rejects_outside_extent(self):
        with self.assertRaises(ValueError):
            self.parse(image(sector=999))

    def test_rejects_unsafe_names(self):
        for name in (b'..', b'/ABS', b'A\\B', b''):
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.parse(image(name=name))

    def test_rejects_inconsistent_endianness(self):
        data = image()
        data[20 * 2048 + 6] = 1
        with self.assertRaises(ValueError):
            self.parse(data)

    def test_rejects_truncated_directory_record(self):
        data = image()
        data[20 * 2048] = 20
        with self.assertRaises(ValueError):
            self.parse(data)


if __name__ == '__main__':
    unittest.main()
