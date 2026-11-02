#!/usr/bin/env python3
"""Generates the small synthetic disc images in this directory.

The images hold only what rcheevos reads to hash each console. Run it again
only if the images need to change; their expected hashes in tests/cli.rs came
from RAHasher.
"""
import pathlib
import struct

HERE = pathlib.Path(__file__).resolve().parent
SECTOR = 2048
SYNC = b"\x00" + b"\xff" * 10 + b"\x00"


def both_endian(value):
    return struct.pack("<I", value) + struct.pack(">I", value)


def dir_record(name, lba, size, flags=0):
    identifier = name.encode()
    length = 33 + len(identifier)
    length += length % 2
    record = bytes([length, 0]) + both_endian(lba) + both_endian(size)
    record += bytes(7) + bytes([flags, 0, 0]) + struct.pack("<H", 1) + struct.pack(">H", 1)
    record += bytes([len(identifier)]) + identifier
    return record.ljust(length, b"\x00")


def iso9660(files, first_sector=b""):
    """Returns 2048-byte sectors: a volume descriptor, root directory and files."""
    root_lba = 18
    next_lba = root_lba + 1
    entries = dir_record("\x00", root_lba, SECTOR, 2) + dir_record("\x01", root_lba, SECTOR, 2)
    contents = []
    for name, data in files:
        entries += dir_record(name, next_lba, len(data))
        contents.append((next_lba, data))
        next_lba += max(1, -(-len(data) // SECTOR))

    sectors = [bytes(SECTOR)] * next_lba
    sectors[0] = first_sector.ljust(SECTOR, b"\x00")
    pvd = bytearray(SECTOR)
    pvd[0:6] = b"\x01CD001"
    pvd[6] = 1
    pvd[156:190] = dir_record("\x00", root_lba, SECTOR, 2)
    sectors[16] = bytes(pvd)
    sectors[17] = b"\xffCD001\x01".ljust(SECTOR, b"\x00")
    sectors[root_lba] = entries.ljust(SECTOR, b"\x00")
    for lba, data in contents:
        for i in range(0, max(len(data), 1), SECTOR):
            sectors[lba + i // SECTOR] = data[i : i + SECTOR].ljust(SECTOR, b"\x00")
    return sectors


def to_raw_mode2(sectors):
    """Wraps 2048-byte sectors in 2352-byte MODE2/form1 sectors."""
    raw = b""
    for lba, data in enumerate(sectors):
        minute, rest = divmod(lba + 150, 60 * 75)
        second, frame = divmod(rest, 75)
        address = bytes(int(str(n), 16) for n in (minute, second, frame))
        raw += SYNC + address + b"\x02" + bytes(8) + data + bytes(280)
    return raw


def write(name, data):
    (HERE / name).write_bytes(data)


# PlayStation: the hash covers the boot executable named by SYSTEM.CNF.
exe = b"PS-X EXE".ljust(0x800, b"\x00") + b"synthetic playstation code".ljust(SECTOR, b"\x00")
exe = exe[:0x1c] + struct.pack("<I", SECTOR) + exe[0x20:]
psx = iso9660(
    [("SYSTEM.CNF;1", b"BOOT = cdrom:\\TEST.EXE;1\r\n"), ("TEST.EXE;1", exe)]
)
write("psx.bin", to_raw_mode2(psx))
write("psx.cue", b'FILE "psx.bin" BINARY\n  TRACK 01 MODE2/2352\n    INDEX 01 00:00:00\n')
write("psx.m3u", b"psx.cue\n")

# A disc that passes for both PlayStation and Sega CD, so hashing it without
# naming a console produces more than one distinct hash.
both = iso9660(
    [("SYSTEM.CNF;1", b"BOOT = cdrom:\\TEST.EXE;1\r\n"), ("TEST.EXE;1", exe)],
    b"SEGADISCSYSTEM  synthetic sega cd".ljust(512, b"."),
)
write("both.bin", to_raw_mode2(both))
write("both.cue", b'FILE "both.bin" BINARY\n  TRACK 01 MODE2/2352\n    INDEX 01 00:00:00\n')

# Sega CD: the hash covers the first 512 bytes of the data track.
segacd = iso9660([], b"SEGADISCSYSTEM  synthetic sega cd".ljust(512, b"."))
write("segacd.iso", b"".join(segacd))
write("segacd.bin", to_raw_mode2(segacd))
write("segacd.gdi", b"1\n1 0 4 2352 segacd.bin 0\n")

# 3DS: a header that claims encrypted content. Without keys it can't be hashed.
ncch = bytearray(0x4000)
ncch[0x100:0x104] = b"NCCH"
ncch[0x1a0:0x1a8] = struct.pack("<II", 1, 1)
write("encrypted.3ds", bytes(ncch))
