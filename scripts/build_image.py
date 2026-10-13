#!/usr/bin/env python3
"""生成教材的 1.44 MiB FAT12 软盘，无挂载、sudo 或第三方 Python 包。

固定布局来自 src/boot/boot.asm 的 BPB：1 引导扇区、2×9 FAT 扇区、
14 根目录扇区、每簇 1 扇区。额外解析 BPB，防止引导器改动后仍写错布局。
"""
from pathlib import Path
import struct

SECTOR = 512
FAT_SIZE = 9 * SECTOR
ROOT = 19 * SECTOR
DATA = 33 * SECTOR
IMAGE_SIZE = 2880 * SECTOR


def fat_entry(fat: bytes, cluster: int) -> int:
    offset = cluster * 3 // 2
    pair = int.from_bytes(fat[offset:offset + 2], "little")
    return (pair >> 4 if cluster & 1 else pair) & 0xFFF


def check(image: bytes, files: list[tuple[bytes, bytes]]) -> None:
    """从目录和 FAT 链还原文件，验证最终磁盘字节与构建输入完全一致。"""
    assert len(image) == IMAGE_SIZE and image[510:512] == b"\x55\xaa"
    fat = image[SECTOR:SECTOR + FAT_SIZE]
    assert fat == image[SECTOR + FAT_SIZE:SECTOR + 2 * FAT_SIZE]
    seen = set()
    for index, (name, expected) in enumerate(files):
        entry = image[ROOT + index * 32:ROOT + (index + 1) * 32]
        assert entry[:11] == name
        cluster, size = struct.unpack_from("<HI", entry, 26)
        assert size == len(expected)
        result = bytearray()
        while cluster < 0xFF8:
            assert 2 <= cluster < 2849 and cluster not in seen
            seen.add(cluster)
            start = DATA + (cluster - 2) * SECTOR
            result.extend(image[start:start + SECTOR])
            cluster = fat_entry(fat, cluster)
        assert bytes(result[:size]) == expected, name
        assert len(result) == ((size + SECTOR - 1) // SECTOR) * SECTOR


def main() -> None:
    boot = Path("bin/boot.bin").read_bytes()
    assert len(boot) == SECTOR and boot[-2:] == b"\x55\xaa"
    assert struct.unpack_from("<HBHBHHBHHH", boot, 11) == (
        512, 1, 1, 2, 224, 2880, 0xF0, 9, 18, 2
    ), "BPB changed; update image builder with bootloader"
    files = [(b"LOADER  BIN", Path("bin/loader.bin").read_bytes()),
             (b"KERNEL  BIN", Path("bin/kernel.bin").read_bytes())]
    # Boot 用 16 位 BX 连续装载 Loader，不支持跨越 64 KiB 段。
    assert 0 < len(files[0][1]) < 65536
    image = bytearray(IMAGE_SIZE)
    image[:SECTOR] = boot
    fat = bytearray(FAT_SIZE)
    fat[:3] = b"\xf0\xff\xff"
    next_cluster = 2
    for index, (name, content) in enumerate(files):
        assert content
        clusters = (len(content) + SECTOR - 1) // SECTOR
        assert next_cluster + clusters <= 2849, "floppy full"
        directory = bytearray(32)
        directory[:11] = name
        directory[11] = 0x20
        struct.pack_into("<HI", directory, 26, next_cluster, len(content))
        image[ROOT + index * 32:ROOT + (index + 1) * 32] = directory
        for i in range(clusters):
            cluster = next_cluster + i
            link = 0xFFF if i == clusters - 1 else cluster + 1
            offset = cluster * 3 // 2
            pair = int.from_bytes(fat[offset:offset + 2], "little")
            pair = (pair & 0x000F) | (link << 4) if cluster & 1 else (pair & 0xF000) | link
            fat[offset:offset + 2] = pair.to_bytes(2, "little")
            block = content[i * SECTOR:(i + 1) * SECTOR]
            start = DATA + (cluster - 2) * SECTOR
            image[start:start + len(block)] = block
        next_cluster += clusters
    image[SECTOR:SECTOR + FAT_SIZE] = fat
    image[SECTOR + FAT_SIZE:SECTOR + 2 * FAT_SIZE] = fat
    check(image, files)
    Path("bin/boot.img").write_bytes(image)
    print(f"FAT12 image verified: {IMAGE_SIZE} bytes; " + ", ".join(
        f"{name.decode().strip()}={len(content)} bytes" for name, content in files))


if __name__ == "__main__":
    main()
