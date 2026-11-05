"""Read-only, bounded UI lifetime observation for the validated 2.7.2 executable."""
import argparse
import ctypes as c
import hashlib
import json
from pathlib import Path
import struct
import time

GAME_SHA256 = 'f9118fea2cfd8fa3901e8060bfadb1552cb2471771ded8a5b662c4e298ffb328'


def active_states(data):
    if len(data) != 0x46:
        raise ValueError('Incomplete CSMenuMan UI state array')
    return [{'index': i, 'created': bool(v & 1), 'visible': bool(v & 2), 'raw': v}
            for i, v in enumerate(data) if v]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--pid', type=int, required=True)
    parser.add_argument('--base', type=lambda value: int(value, 0), required=True)
    parser.add_argument('--exe', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--seconds', type=int, default=120)
    args = parser.parse_args()
    if not 1 <= args.seconds <= 300:
        parser.error('Observation must last between 1 and 300 seconds')
    if hashlib.sha256(args.exe.read_bytes()).hexdigest() != GAME_SHA256:
        raise RuntimeError('Unknown game executable; no process access attempted')
    import pefile
    pe = pefile.PE(str(args.exe), fast_load=True)
    kernel = c.WinDLL('kernel32', use_last_error=True)
    kernel.OpenProcess.argtypes = [c.c_uint, c.c_bool, c.c_uint]
    kernel.OpenProcess.restype = c.c_void_p
    kernel.ReadProcessMemory.argtypes = [c.c_void_p, c.c_void_p, c.c_void_p, c.c_size_t, c.POINTER(c.c_size_t)]
    kernel.CloseHandle.argtypes = [c.c_void_p]
    handle = kernel.OpenProcess(0x410, False, args.pid)
    if not handle:
        raise OSError(c.get_last_error())

    def read(address, size):
        if address < 0x10000 or size > 32 * 1024 * 1024:
            return None
        buffer = c.create_string_buffer(size)
        got = c.c_size_t()
        if kernel.ReadProcessMemory(handle, address, buffer, size, c.byref(got)) and got.value == size:
            return buffer.raw
        return None

    def ptr(address):
        raw = read(address, 8)
        return struct.unpack('<Q', raw)[0] if raw else 0

    image_end = args.base + pe.OPTIONAL_HEADER.SizeOfImage
    names = {}

    def rtti(obj):
        vtable = ptr(obj)
        if not args.base <= vtable < image_end:
            return None
        if vtable not in names:
            col = ptr(vtable - 8)
            raw = read(col, 24) if args.base <= col < image_end else None
            type_rva = struct.unpack_from('<I', raw, 12)[0] if raw else 0
            name = read(args.base + type_rva + 16, 128) if 0 < type_rva < pe.OPTIONAL_HEADER.SizeOfImage else None
            names[vtable] = name.split(b'\0')[0].decode('ascii', errors='replace') if name else None
        return names[vtable]

    try:
        singleton = None
        for section in pe.sections:
            if section.Name.rstrip(b'\0') != b'.data':
                continue
            data = read(args.base + section.VirtualAddress, section.Misc_VirtualSize)
            if data is None:
                raise RuntimeError('Cannot read module data')
            for offset in range(0, len(data) - 7, 8):
                obj = struct.unpack_from('<Q', data, offset)[0]
                if rtti(obj) in ('.?AVCSMenuManImp@CS@@', '.?AVCSMenuMan@CS@@'):
                    singleton = args.base + section.VirtualAddress + offset
                    break
        if singleton is None:
            raise RuntimeError('CSMenuMan RTTI singleton not found')
        args.out.parent.mkdir(parents=True, exist_ok=True)
        with args.out.open('x', encoding='utf-8') as output:
            output.write(json.dumps({'pid': args.pid, 'base': hex(args.base), 'singleton_rva': hex(singleton-args.base), 'read_only': True}) + '\n')
            started, previous = time.monotonic(), None
            print('READY: read-only menu observation', flush=True)
            while time.monotonic() - started < args.seconds:
                obj = ptr(singleton)
                if rtti(obj) not in ('.?AVCSMenuManImp@CS@@', '.?AVCSMenuMan@CS@@'):
                    raise RuntimeError('Menu manager disappeared; observation stopped')
                states = read(obj + 0x90, 0x46)
                if states is None:
                    raise RuntimeError('Menu state array unreadable')
                popup, job = ptr(obj + 0x80), ptr(obj + 0x88)
                snapshot = {'manager': hex(obj), 'states': active_states(states),
                            'popup': {'address': hex(popup), 'type': rtti(popup)},
                            'window_job': {'address': hex(job), 'type': rtti(job)}}
                if snapshot != previous:
                    output.write(json.dumps({'elapsed': round(time.monotonic()-started, 3), **snapshot}) + '\n')
                    output.flush()
                    previous = snapshot
                time.sleep(0.1)
        print('DONE:', args.out, flush=True)
    finally:
        kernel.CloseHandle(handle)
        pe.close()


if __name__ == '__main__':
    main()
