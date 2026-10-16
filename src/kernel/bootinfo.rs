//! 第 7 章：固件内存图是输入，不能在验证前把 type=1 的字节交给分配器。
//! Loader 将 BIOS 的可变长返回规范成 20 字节条目；交接头最后置 complete，
//! 区分“完整表”与失败时留下的半张表。这里只校验与复制，不建立页表或分配 RAM。
use crate::{console, kprint, memory::PAGE_OFFSET};
use core::{mem::size_of, ptr::read_unaligned};

pub const MAX_ENTRIES: usize = 32;
const HEADER: usize = 0x4ff0;
const ENTRIES: usize = 0x5000;
const MAGIC: u32 = 0x3032_3845; // 小端字节为 E820。

#[repr(C)]
#[derive(Clone, Copy)]
struct Header {
    magic: u32,
    version: u16,
    entry_size: u16,
    count: u16,
    capacity: u16,
    flags: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct E820Entry {
    pub address: u64,
    pub length: u64,
    pub kind: u32,
}
impl E820Entry {
    pub const EMPTY: Self = Self {
        address: 0,
        length: 0,
        kind: 0,
    };
}

pub struct MemoryMap {
    pub entries: [E820Entry; MAX_ENTRIES],
    pub count: usize,
}

impl MemoryMap {
    pub fn contains_usable(&self, start: usize, end: usize) -> bool {
        start < end
            && self.entries[..self.count].iter().any(|entry| {
                entry.kind == 1
                    && entry.length != 0
                    && entry.address <= start as u64
                    && entry.address + entry.length >= end as u64
            })
    }
}

const _: () = assert!(size_of::<Header>() == 16 && HEADER + 16 == ENTRIES);
const _: () = assert!(core::mem::offset_of!(Header, count) == 8);
const _: () = assert!(core::mem::offset_of!(Header, flags) == 12);
const _: () = assert!(size_of::<E820Entry>() == 20);
const _: () = assert!(ENTRIES + MAX_ENTRIES * 20 <= 0x5300);

impl Header {
    fn validate(self) -> Result<usize, &'static str> {
        if self.magic != MAGIC
            || self.version != 1
            || self.entry_size != 20
            || self.capacity as usize != MAX_ENTRIES
            || self.flags != 1
        {
            return Err("invalid or incomplete E820 handoff");
        }
        if self.count == 0 || self.count as usize > MAX_ENTRIES {
            return Err("invalid E820 count");
        }
        Ok(self.count as usize)
    }
}

fn validate_entries(entries: &[E820Entry]) -> Result<(), &'static str> {
    for (index, entry) in entries.iter().enumerate() {
        let Some(end) = entry.address.checked_add(entry.length) else {
            return Err("E820 address overflow");
        };
        if entry.length == 0 {
            continue;
        }
        // 输入可无序，未知类型保留为不可分配。这里采取拒绝重叠的保守策略，
        // 尤其不能让一个 type=1 区间覆盖 type=2/ACPI/NVS 等保留区。
        // 通用固件修复器可以拆分并按类型优先级覆盖，本节不悄悄猜测这些规则。
        for previous in &entries[..index] {
            let previous_end = previous.address + previous.length; // 前面的条目已验溢出。
            if previous.length != 0 && entry.address < previous_end && previous.address < end {
                return Err("overlapping E820 ranges");
            }
        }
    }
    Ok(())
}

/// # Safety
/// Loader 已交接低端缓冲，head.S 仍将其高半区别名映射；单核 IF=0，无固件写入者。
pub unsafe fn read_memory_map() -> MemoryMap {
    // SAFETY: 固定 16 字节头可读；先验格式和容量，再依有效数量读取条目。
    let header = unsafe { read_unaligned((PAGE_OFFSET + HEADER) as *const Header) };
    let count = header.validate().expect("E820 handoff rejected");
    let mut map = MemoryMap {
        entries: [E820Entry::EMPTY; MAX_ENTRIES],
        count,
    };
    for (index, entry) in map.entries[..count].iter_mut().enumerate() {
        // SAFETY: count <= 32，完整条目均在 [0x5000,0x5280)，无 Rust 引用指向固件内存。
        *entry =
            unsafe { read_unaligned((PAGE_OFFSET + ENTRIES + index * 20) as *const E820Entry) };
    }
    validate_entries(&map.entries[..count]).expect("E820 map rejected");
    // 页表/Loader/栈有固定引导位置，E820 与这些假定不符时拒绝继续分配。
    // 这是 Rust 接管后的复核，不能追溯保护已经完成的 BIOS/Loader 内存写入。
    for (start, end) in [
        (HEADER, 0x5318),
        (0x6000, 0x10000),
        (0x10000, 0x20000),
        (0x90000, 0x93000),
    ] {
        assert!(
            map.contains_usable(start, end),
            "bootstrap layout is not usable RAM"
        );
    }
    #[cfg(feature = "memory-check")]
    verify();
    kprint!(
        console::GREEN,
        "E820 handoff OK: version=1, entries={count}, stride=20; complete, no overlaps\n"
    );
    kprint!(
        console::GREEN,
        "Boot memory layout OK: handoff, stack, VBE scratch, loader, temporary page tables\n"
    );
    map
}

/// 实现后的输入验证：仅用局部值，不覆盖真实固件交接或分配器状态。
#[cfg(feature = "memory-check")]
fn verify() {
    let good = Header {
        magic: MAGIC,
        version: 1,
        entry_size: 20,
        count: 32,
        capacity: 32,
        flags: 1,
    };
    assert_eq!(good.validate(), Ok(32));
    for bad in [
        Header { magic: 0, ..good },
        Header { version: 2, ..good },
        Header {
            entry_size: 24,
            ..good
        },
        Header {
            capacity: 31,
            ..good
        },
        Header { flags: 0, ..good },
        Header { count: 0, ..good },
        Header { count: 33, ..good },
    ] {
        assert!(bad.validate().is_err());
    }
    let ram = E820Entry {
        address: 0x100000,
        length: 0x400000,
        kind: 1,
    };
    let reserved = E820Entry {
        address: 0x500000,
        length: 0x1000,
        kind: 2,
    };
    // 无序、相邻端点、未知保留类型和零长度均合法；重叠与端地址溢出必须拒绝。
    assert!(validate_entries(&[reserved, ram, E820Entry::EMPTY]).is_ok());
    assert!(
        validate_entries(&[
            E820Entry {
                kind: 99,
                ..reserved
            },
            ram
        ])
        .is_ok()
    );
    assert!(
        validate_entries(&[
            ram,
            E820Entry {
                address: 0x200000,
                ..reserved
            }
        ])
        .is_err()
    );
    assert!(validate_entries(&[ram, ram]).is_err());
    assert!(
        validate_entries(&[E820Entry {
            address: u64::MAX,
            length: 1,
            kind: 1
        }])
        .is_err()
    );
    let mut map = MemoryMap {
        entries: [E820Entry::EMPTY; MAX_ENTRIES],
        count: 2,
    };
    map.entries[..2].copy_from_slice(&[reserved, ram]);
    assert!(map.contains_usable(0x100000, 0x500000));
    assert!(!map.contains_usable(0x100000, 0x500001));
    assert!(!map.contains_usable(0x500000, 0x500001));
    assert!(!map.contains_usable(0x100000, 0x100000));
    kprint!(
        console::GREEN,
        "E820 input checks OK: header, overflow, overlaps, ordering, reserved ranges\n"
    );
}
