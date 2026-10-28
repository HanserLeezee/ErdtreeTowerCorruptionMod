"""Minimal Elden Ring regulation.bin reader/writer (AES -> DCX ZSTD -> BND4 -> PARAM).

Mirrors SoulsFormats (RegulationDecryptor, DCX ZSTD, BND4, PARAM) closely enough that an
unchanged param re-serializes byte-for-byte; every write path is round-trip checked.
"""
import struct, re, xml.etree.ElementTree as ET
from Crypto.Cipher import AES
import zstandard

KEY = bytes([0x99,0xBF,0xFC,0x36,0x6A,0x6B,0xC8,0xC6,0xF5,0x82,0x7D,0x09,0x36,0x02,0xD6,0x76,
             0xC4,0x28,0x92,0xA0,0x1C,0x20,0x7F,0xB0,0x24,0xD3,0xAF,0x4E,0x49,0x3F,0xEF,0x99])

# ---------------- encryption ----------------
def decrypt(data: bytes) -> bytes:
    iv, body = data[:16], data[16:]
    if len(body) % 16: body += b'\0' * (16 - len(body) % 16)
    return AES.new(KEY, AES.MODE_CBC, iv).decrypt(body)

def encrypt(plain: bytes) -> bytes:
    pad = 16 - len(plain) % 16
    plain += bytes([pad]) * pad
    iv = b'\0' * 16
    return iv + AES.new(KEY, AES.MODE_CBC, iv).encrypt(plain)

# ---------------- DCX (ZSTD) ----------------
def dcx_read(data: bytes):
    assert data[:4] == b'DCX\0' and data[0x28:0x2C] == b'ZSTD', data[:0x30]
    usize, csize = struct.unpack_from('>II', data, 0x1C)
    level = data[0x30]
    raw = zstandard.ZstdDecompressor().decompress(data[0x4C:0x4C + csize], max_output_size=usize)
    assert len(raw) == usize
    return raw, level

def dcx_write(raw: bytes, level: int) -> bytes:
    # Match the game's own regulation frames: 64 KiB window, no content size, no checksum.
    params = zstandard.ZstdCompressionParameters.from_level(level, window_log=16, write_content_size=False, write_checksum=False)
    comp = zstandard.ZstdCompressor(compression_params=params).compress(raw)
    h = b'DCX\0' + struct.pack('>iiiii', 0x11000, 0x18, 0x24, 0x44, 0x4C)
    h += b'DCS\0' + struct.pack('>II', len(raw), len(comp))
    h += b'DCP\0' + b'ZSTD' + struct.pack('>i', 0x20) + bytes([level, 0, 0, 0]) + struct.pack('>iiii', 0, 0, 0, 0x10100)
    h += b'DCA\0' + struct.pack('>i', 8)
    assert len(h) == 0x4C
    out = h + comp
    if len(out) % 16: out += b'\0' * (16 - len(out) % 16)
    return out

# ---------------- BND4 ----------------
def _rev(b):
    return int('{:08b}'.format(b)[::-1], 2)

class Bnd4:
    def __init__(self, raw: bytes):
        assert raw[:4] == b'BND4'
        self.raw = bytearray(raw)
        big = raw[9] != 0
        assert not big
        bit_big = not raw[10]
        count, = struct.unpack_from('<i', raw, 0x0C)
        hdr_size, = struct.unpack_from('<q', raw, 0x20)
        self.unicode = raw[0x30] != 0
        fmt = raw[0x31]
        reverse = bit_big or ((fmt & 1) and not (fmt & 0x80))
        fmt = fmt if reverse else _rev(fmt)
        self.fmt = fmt
        has_ids, has_names = bool(fmt & 2), bool(fmt & 0xC)
        long_off, comp = bool(fmt & 0x10), bool(fmt & 0x20)
        self.entries = []
        for i in range(count):
            p = 0x40 + i * hdr_size
            q = p + 8
            csize_pos = q; q += 8
            usize_pos = None
            if comp: usize_pos = q; q += 8
            off_pos = q; off_fmt = '<q' if long_off else '<I'; q += 8 if long_off else 4
            fid = None
            if has_ids: fid, = struct.unpack_from('<i', raw, q); q += 4
            name = None
            if has_names:
                noff, = struct.unpack_from('<I', raw, q); q += 4
                if self.unicode:
                    end = noff
                    while raw[end:end + 2] != b'\0\0': end += 2
                    name = raw[noff:end].decode('utf-16-le')
                else:
                    name = raw[noff:raw.index(b'\0', noff)].decode('shift_jis')
            size, = struct.unpack_from('<q', raw, csize_pos)
            off, = struct.unpack_from(off_fmt, raw, off_pos)
            self.entries.append(dict(name=name, id=fid, size=size, offset=off, csize_pos=csize_pos,
                                     usize_pos=usize_pos, off_pos=off_pos, off_fmt=off_fmt))

    def find(self, stem):
        hits = [e for e in self.entries if e['name'] and re.split(r'[\\/]', e['name'])[-1].rsplit('.', 1)[0] == stem]
        assert len(hits) == 1, (stem, len(hits))
        return hits[0]

    def data(self, entry):
        return bytes(self.raw[entry['offset']:entry['offset'] + entry['size']])

    def replace(self, entry, new: bytes):
        """Splice new bytes in place of the entry; later file offsets shift by a multiple of 16."""
        old_off, old_size = entry['offset'], entry['size']
        slot = old_size + (-old_size % 16)
        nslot = len(new) + (-len(new) % 16)
        # keep the original gap after the file if it was already aligned
        end = old_off + slot
        if end > len(self.raw): end = old_off + old_size
        block = new + b'\0' * (nslot - len(new))
        delta = len(block) - (end - old_off)
        assert delta % 16 == 0
        self.raw[old_off:end] = block
        for e in self.entries:
            if e is not entry and e['offset'] > old_off:
                e['offset'] += delta
                struct.pack_into(e['off_fmt'], self.raw, e['off_pos'], e['offset'])
        entry['size'] = len(new)
        struct.pack_into('<q', self.raw, entry['csize_pos'], len(new))
        if entry['usize_pos'] is not None:
            struct.pack_into('<q', self.raw, entry['usize_pos'], len(new))

# ---------------- PARAMDEF ----------------
SIZES = {'s8': 1, 'u8': 1, 's16': 2, 'u16': 2, 's32': 4, 'u32': 4, 'b32': 4, 'f32': 4,
         'angle32': 4, 'f64': 8, 'dummy8': 1, 'fixstr': 1, 'fixstrW': 2}
FMT = {'s8': '<b', 'u8': '<B', 's16': '<h', 'u16': '<H', 's32': '<i', 'u32': '<I', 'b32': '<i',
       'f32': '<f', 'angle32': '<f', 'f64': '<d'}

class Field:
    pass

def load_def(path):
    root = ET.parse(path).getroot()
    fields = []
    offset = 0
    bit_unit = None  # (type, start_offset, bits_used)
    for node in root.find('Fields'):
        d = node.get('Def')
        m = re.match(r'(\w+)\s+(\w+)(?:\[(\d+)\])?(?::\s*(\d+))?(?:\s*=\s*(.+))?$', d.strip())
        assert m, d
        t, name, count, bits, default = m.groups()
        f = Field(); f.type = t; f.name = name; f.bits = int(bits) if bits else None
        f.count = int(count) if count else 1
        f.default = default
        if f.bits is not None and t != 'dummy8':
            size = SIZES[t]
            if bit_unit and bit_unit[0] == t and bit_unit[2] + f.bits <= size * 8:
                f.offset, f.bit = bit_unit[1], bit_unit[2]
                bit_unit = (t, bit_unit[1], bit_unit[2] + f.bits)
            else:
                f.offset, f.bit = offset, 0
                bit_unit = (t, offset, f.bits); offset += size
        elif f.bits is not None and t == 'dummy8':
            # bit padding inside a u8 unit
            if bit_unit and bit_unit[0] in ('u8', 'dummy8') and bit_unit[2] + f.bits <= 8:
                f.offset, f.bit = bit_unit[1], bit_unit[2]
                bit_unit = (bit_unit[0], bit_unit[1], bit_unit[2] + f.bits)
            else:
                f.offset, f.bit = offset, 0
                bit_unit = ('u8', offset, f.bits); offset += 1
        else:
            bit_unit = None
            f.offset, f.bit = offset, None
            offset += SIZES[t] * f.count
        fields.append(f)
    return {f.name: f for f in fields}, offset

def get(row: bytes, f):
    if f.bit is not None:
        v = struct.unpack_from(FMT['u8' if f.type == 'dummy8' else f.type], row, f.offset)[0]
        return (v >> f.bit) & ((1 << f.bits) - 1)
    return struct.unpack_from(FMT[f.type], row, f.offset)[0]

def put(row: bytearray, f, value):
    if f.bit is not None:
        t = FMT[f.type]
        v = struct.unpack_from(t, row, f.offset)[0]
        mask = ((1 << f.bits) - 1) << f.bit
        v = (v & ~mask) | ((int(value) << f.bit) & mask)
        struct.pack_into(t, row, f.offset, v)
    else:
        if f.type in ('f32', 'angle32', 'f64'): value = float(value)
        else: value = int(value)
        struct.pack_into(FMT[f.type], row, f.offset, value)

# ---------------- PARAM ----------------
class Param:
    def __init__(self, data: bytes, row_size: int):
        self.src = data
        self.row_size = row_size
        self.f2d, self.f2e, self.fver = data[0x2D], data[0x2E], data[0x2F]
        assert data[0x2C] == 0
        assert self.f2d & 0x04  # LongDataOffset (ER)
        assert self.f2d & 0x80  # OffsetParamType
        self.unk06, self.dver, count = struct.unpack_from('<hhH', data, 6)
        ptype_off, = struct.unpack_from('<q', data, 0x10)
        self.ptype = data[ptype_off:data.index(b'\0', ptype_off)].decode('ascii')
        self.unicode = bool(self.f2e & 1)
        rows_start = 0x40
        self.rows = []
        for i in range(count):
            rid, _, doff, noff = struct.unpack_from('<iiqq', data, rows_start + i * 24)
            row = bytes(data[doff:doff + row_size])
            if self.unicode:
                end = noff
                while data[end:end + 2] != b'\0\0': end += 2
                name = data[noff:end].decode('utf-16-le')
            else:
                name = data[noff:data.index(b'\0', noff)].decode('shift_jis')
            self.rows.append([rid, row, name])

    def write(self) -> bytes:
        out = bytearray()
        out += b'\0' * 4            # strings offset (fill)
        out += struct.pack('<h', 0)
        out += struct.pack('<hhH', self.unk06, self.dver, len(self.rows))
        out += struct.pack('<i', 0) + b'\0' * 8   # paramtype offset (fill at 0x10)
        out += b'\0' * 0x14
        out += bytes([0, self.f2d, self.f2e, self.fver])
        out += b'\0' * 8                          # data start (fill at 0x30)
        out += b'\0' * 8
        assert len(out) == 0x40
        hdr = len(out)
        out += b'\0' * (24 * len(self.rows))
        data_start = len(out)
        struct.pack_into('<q', out, 0x30, data_start)
        for i, (rid, row, name) in enumerate(self.rows):
            assert len(row) == self.row_size
            struct.pack_into('<iiq', out, hdr + i * 24, rid, 0, len(out))
            out += row
        struct.pack_into('<I', out, 0, len(out))
        struct.pack_into('<q', out, 0x10, len(out))
        out += self.ptype.encode('ascii') + b'\0'
        strings = {'': len(out)}
        out += b'\0\0'
        for i, (rid, row, name) in enumerate(self.rows):
            name = name or ''
            off = strings.get(name, 0)
            if off == 0:
                off = len(out)
                out += name.encode('utf-16-le' if self.unicode else 'shift_jis') + (b'\0\0' if self.unicode else b'\0')
                strings[name] = off
            struct.pack_into('<q', out, hdr + i * 24 + 16, off)
        out += b'\0\0'
        return bytes(out)

    def row(self, rid):
        hits = [r for r in self.rows if r[0] == rid]
        return hits[0] if hits else None

# ---------------- top level ----------------
class Regulation:
    def __init__(self, path):
        self.dcx = decrypt(open(path, 'rb').read())
        raw, self.level = dcx_read(self.dcx)
        self.bnd = Bnd4(raw)

    def save(self, path):
        open(path, 'wb').write(encrypt(dcx_write(bytes(self.bnd.raw), self.level)))
