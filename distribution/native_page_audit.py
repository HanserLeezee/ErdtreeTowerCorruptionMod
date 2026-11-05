"""Offline, exact-build native page call graph. Never opens a game process."""
import argparse
import bisect
import hashlib
import json
from pathlib import Path
import struct

GAME_SHA256 = 'f9118fea2cfd8fa3901e8060bfadb1552cb2471771ded8a5b662c4e298ffb328'
TARGETS = [0x950d00, 0x808d10, 0x808e40, 0x809960, 0x9601e0,
           0x94dbc0, 0x91d0d0, 0x9539d0, 0x748180, 0x747d00, 0x93ce60,
           0x93dbe0, 0x94ec40]


def unwind_ranges(raw):
    if len(raw) % 12:
        raise ValueError('Truncated x64 exception directory')
    return sorted((start, end, unwind) for start, end, unwind in struct.iter_unpack('<III', raw)
                  if start and start < end)


def containing(ranges, starts, address):
    index = bisect.bisect_right(starts, address) - 1
    return ranges[index] if index >= 0 and address < ranges[index][1] else None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--exe', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    if hashlib.sha256(args.exe.read_bytes()).hexdigest() != GAME_SHA256:
        raise RuntimeError('Unsupported executable; no address assumptions applied')
    import capstone
    import pefile
    pe = pefile.PE(str(args.exe), fast_load=True)
    exception = pe.OPTIONAL_HEADER.DATA_DIRECTORY[3]
    ranges = unwind_ranges(pe.get_data(exception.VirtualAddress, exception.Size))
    starts = [entry[0] for entry in ranges]
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
    args.out.mkdir(parents=True, exist_ok=True)
    decoded = {}

    def instructions(address):
        entry = containing(ranges, starts, address)
        if entry is None:
            raise ValueError(f'No unwind boundary for {address:x}')
        start, end, _ = entry
        if start not in decoded:
            decoded[start] = list(decoder.disasm(pe.get_data(start, end-start), start))
        return entry, decoded[start]

    report = {'game_sha256': GAME_SHA256, 'unwind_entries': len(ranges),
              'functions': [], 'verified_direct_calls': [], 'limitations':
              ['Direct calls only; indirect and tail calls are not enumerated.',
               'Function boundaries do not prove ABI, ownership, or permission to invoke.']}
    for target in TARGETS:
        entry, code = instructions(target)
        report['functions'].append({'rva': hex(target), 'begin': hex(entry[0]), 'end': hex(entry[1]),
                                    'calls': [i.op_str for i in code if i.mnemonic == 'call']})
    for section in pe.sections:
        if not section.Characteristics & 0x20000000:
            continue
        data, pos = section.get_data(), 0
        while True:
            pos = data.find(b'\xe8', pos)
            if pos < 0 or pos+5 > len(data):
                break
            address = section.VirtualAddress + pos
            target = address + 5 + struct.unpack_from('<i', data, pos+1)[0]
            pos += 1
            if target not in TARGETS or containing(ranges, starts, address) is None:
                continue
            entry, code = instructions(address)
            if any(i.address == address and i.mnemonic == 'call' for i in code):
                report['verified_direct_calls'].append({'caller': hex(entry[0]), 'site': hex(address), 'target': hex(target)})
    for start, code in decoded.items():
        (args.out / f'{start:08x}.asm').write_text('\n'.join(f'{i.address:08x} {i.mnemonic} {i.op_str}' for i in code), encoding='utf-8')
    (args.out / 'report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    pe.close()
    print(json.dumps({'unwind_entries': len(ranges), 'decoded_functions': len(decoded),
                      'direct_calls': len(report['verified_direct_calls']), 'report': str(args.out / 'report.json')}))


if __name__ == '__main__':
    main()
