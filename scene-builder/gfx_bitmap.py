"""Embed full-screen JPEG artwork into an uncompressed GFx (SWF) movie without adding ActionScript classes.

For every picture this adds a DefineBitsJPEG2 tag, a DefineShape rectangle filled with that bitmap, and a PlaceObject2 that puts the
shape on frame 1 of a chosen timeline (a DefineSprite, or the root timeline) under an instance name. The theme script then finds the
instance with getChildByName and moves it to the bottom (setChildIndex 0), so native buttons and loading widgets stay on top.
FFDec's script import can only replace existing classes, which is why the art is placed on the timeline instead of instantiated.
"""
import io
import struct

from PIL import Image

DEFINE_BITS_JPEG2, DEFINE_SHAPE, PLACE_OBJECT2, PLACE_OBJECT3, DEFINE_SPRITE, SHOW_FRAME, END, SYMBOL_CLASS = 21, 2, 26, 70, 39, 1, 0, 76
CHARACTER_TAGS = {2, 6, 7, 10, 11, 14, 20, 21, 22, 32, 33, 34, 35, 36, 37, 39, 46, 48, 60, 75, 83, 84, 87, 90, 91, 1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008, 1009}


class Bits:
    def __init__(self):
        self.bits = []

    def unsigned(self, value, count):
        self.bits += [(value >> (count - 1 - i)) & 1 for i in range(count)]

    def signed(self, value, count):
        self.unsigned(value & ((1 << count) - 1), count)

    def data(self):
        padded = self.bits + [0] * (-len(self.bits) % 8)
        return bytes(int(''.join(map(str, padded[i:i + 8])), 2) for i in range(0, len(padded), 8))


def signed_bits(*values):
    return max(max(abs(v).bit_length() + 1 for v in values), 2)


def rect(xmin, xmax, ymin, ymax):
    b = Bits(); n = signed_bits(xmin, xmax, ymin, ymax)
    b.unsigned(n, 5)
    for v in (xmin, xmax, ymin, ymax):
        b.signed(v, n)
    return b.data()


def matrix(scale_x=None, scale_y=None, tx=0, ty=0, bits=None):
    b = bits or Bits()
    if scale_x is None:
        b.unsigned(0, 1)
    else:
        sx, sy = round(scale_x * 65536), round(scale_y * 65536)
        n = signed_bits(sx, sy); b.unsigned(1, 1); b.unsigned(n, 5); b.signed(sx, n); b.signed(sy, n)
    b.unsigned(0, 1)
    n = signed_bits(tx, ty) if tx or ty else 0
    b.unsigned(n, 5)
    if n:
        b.signed(tx, n); b.signed(ty, n)
    return b.data()


def tag(code, body):
    if len(body) < 63 and code not in (DEFINE_BITS_JPEG2,):
        return struct.pack('<H', code << 6 | len(body)) + body
    return struct.pack('<HI', code << 6 | 63, len(body)) + body


def parse(data):
    if data[:3] != b'GFX':
        raise ValueError('Expected uncompressed GFX')
    head = 8 + ((5 + (data[8] >> 3) * 4 + 7) // 8) + 4
    return data[:head], list(read_tags(data, head, struct.unpack_from('<I', data, 4)[0]))


def read_tags(data, pos, end):
    while pos < end:
        header = struct.unpack_from('<H', data, pos)[0]; pos += 2
        size = header & 63
        if size == 63:
            size = struct.unpack_from('<I', data, pos)[0]; pos += 4
        yield header >> 6, data[pos:pos + size]
        pos += size


def build(head, tags):
    body = b''.join(tag(code, data) for code, data in tags)
    out = bytearray(head + body)
    struct.pack_into('<I', out, 4, len(out))
    return bytes(out)


def jpeg(path, size=(1920, 1080), quality=88):
    image = Image.open(path).convert('RGB').resize(size, Image.LANCZOS)
    buffer = io.BytesIO(); image.save(buffer, 'JPEG', quality=quality, optimize=True, progressive=False)
    return buffer.getvalue()


def bitmap_shape(shape_id, bitmap_id, width, height, image_size):
    """A width x height (pixels) rectangle at the origin, filled with the bitmap scaled to fit."""
    w, h = width * 20, height * 20
    fill = bytes([1, 0x41]) + struct.pack('<H', bitmap_id) + matrix(20 * width / image_size[0], 20 * height / image_size[1])
    b = Bits(); b.unsigned(1, 4); b.unsigned(0, 4)              # one fill style, no line styles
    b.unsigned(0, 1); b.unsigned(0b00111, 5)                    # style change: new fill style 1, fill style 0, move to
    b.unsigned(0, 5)                                            # move to (0, 0) with 0 bits
    b.unsigned(0, 1); b.unsigned(1, 1)                          # fill style 0 = none, fill style 1 = the bitmap
    for dx, dy in ((w, 0), (0, h), (-w, 0), (0, -h)):
        n = signed_bits(dx, dy)
        b.unsigned(1, 1); b.unsigned(1, 1); b.unsigned(n - 2, 4)
        b.unsigned(0, 1)                                        # not a general line
        b.unsigned(1 if dx == 0 else 0, 1)                      # vertical?
        b.signed(dy if dx == 0 else dx, n)
    b.unsigned(0, 6)                                            # end of shape
    return struct.pack('<H', shape_id) + rect(0, w, 0, h) + fill + bytes([0]) + b.data()


def place(depth, character, name, tx=0, ty=0):
    return bytes([0x26]) + struct.pack('<HH', depth, character) + matrix(tx=tx * 20, ty=ty * 20) + name.encode() + b'\0'


def symbols(tags):
    result = {}
    for code, data in tags:
        if code == SYMBOL_CLASS:
            count = struct.unpack_from('<H', data)[0]; pos = 2
            for _ in range(count):
                cid = struct.unpack_from('<H', data, pos)[0]; end = data.index(0, pos + 2)
                result[data[pos + 2:end].decode('utf-8', 'replace')] = cid; pos = end + 1
    return result


def depths(tags):
    used = set()
    for code, data in tags:
        if code in (PLACE_OBJECT2, PLACE_OBJECT3):
            used.add(struct.unpack_from('<H', data, 1 if code == PLACE_OBJECT2 else 2)[0])
    return used


def max_character(tags):
    return max([struct.unpack_from('<H', data)[0] for code, data in tags if code in CHARACTER_TAGS] + [0])


def embed(data, pictures, timeline=None, offset=(0, 0)):
    """pictures: [(instance name, jpeg bytes)]. timeline: class name of the target DefineSprite, or None for the root timeline.
    offset: where the top-left corner goes in the timeline's coordinates (the title timeline is centred, so (-960, -540))."""
    head, tags = parse(data)
    next_id = max_character(tags) + 1
    definitions, placements = [], []
    for name, picture in pictures:
        with Image.open(io.BytesIO(picture)) as image:
            size = image.size
        bitmap, shape = next_id, next_id + 1; next_id += 2
        definitions += [(DEFINE_BITS_JPEG2, struct.pack('<H', bitmap) + picture), (DEFINE_SHAPE, bitmap_shape(shape, bitmap, 1920, 1080, size))]
        placements.append((name, shape))
    if timeline is None:
        first = next(i for i, (code, _) in enumerate(tags) if code == SHOW_FRAME)
        depth = max(depths(tags[:first]) | {0}) + 1
        added = [(PLACE_OBJECT2, place(depth + i, shape, name, *offset)) for i, (name, shape) in enumerate(placements)]
        tags = tags[:first] + definitions + added + tags[first:]
    else:
        target = symbols(tags)[timeline]
        index = next(i for i, (code, body) in enumerate(tags) if code == DEFINE_SPRITE and struct.unpack_from('<H', body)[0] == target)
        body = tags[index][1]; inner = list(read_tags(body, 4, len(body)))
        first = next(i for i, (code, _) in enumerate(inner) if code == SHOW_FRAME)
        depth = max(depths(inner) | {0}) + 1
        inner = inner[:first] + [(PLACE_OBJECT2, place(depth + i, shape, name, *offset)) for i, (name, shape) in enumerate(placements)] + inner[first:]
        sprite = body[:4] + b''.join(tag(code, part) for code, part in inner)
        tags = tags[:index] + definitions + [(DEFINE_SPRITE, sprite)] + tags[index + 1:]
    return build(head, tags)


def verify(before, after, pictures, timeline=None):
    """Everything is unchanged except the added definitions, the target timeline's frame 1 and nothing else."""
    _, old = parse(before); _, new = parse(after)
    added = [t for t in new if t not in old]
    removed = [t for t in old if t not in new]
    jpegs = [t for t in added if t[0] == DEFINE_BITS_JPEG2]; shapes = [t for t in added if t[0] == DEFINE_SHAPE]
    rest = [t for t in added if t[0] not in (DEFINE_BITS_JPEG2, DEFINE_SHAPE)]
    if len(jpegs) != len(pictures) or len(shapes) != len(pictures):
        raise ValueError('Artwork definitions missing')
    if timeline is None:
        if removed or len(rest) != len(pictures) or any(code != PLACE_OBJECT2 for code, _ in rest):
            raise ValueError('Unexpected root timeline change')
    else:
        if len(removed) != 1 or len(rest) != 1 or removed[0][0] != DEFINE_SPRITE or rest[0][0] != DEFINE_SPRITE:
            raise ValueError('Unexpected sprite change')
        old_inner = list(read_tags(removed[0][1], 4, len(removed[0][1]))); new_inner = list(read_tags(rest[0][1], 4, len(rest[0][1])))
        if [t for t in new_inner if t not in old_inner] != [t for t in new_inner if t[0] == PLACE_OBJECT2 and t not in old_inner] or len(new_inner) != len(old_inner) + len(pictures):
            raise ValueError('Unexpected timeline content change')
    return len(new)
