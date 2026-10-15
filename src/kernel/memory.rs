//! 第 4 章初级内存管理：E820 → 2 MiB 物理页 → 位图/Page/Zone。
//! 建立描述与预留记录，提供连续物理页分配；不会把所有 RAM 加入页表。
mod allocator;
use crate::{arch, console, kprint};
#[cfg(feature = "memory-check")]
pub use allocator::verify;
pub use allocator::{AllocationArea, PageRun, allocate_pages, free_pages};
use core::{
    mem::size_of,
    ptr::{addr_of, addr_of_mut, null_mut, read_unaligned},
};

pub const PAGE_OFFSET: usize = 0xffff800000000000;
pub const PAGE_2M_SIZE: usize = 1 << 21;
const PAGE_4K_SIZE: usize = 1 << 12;
const DIRECT_RAM_END: usize = 10 * 1024 * 1024;
const MAX_ENTRIES: usize = 32;
const MAX_ZONES: usize = MAX_ENTRIES;

#[allow(dead_code)]
pub mod flags {
    pub const PAGE_TABLE_MAPPED: u64 = 1 << 0;
    pub const KERNEL_INIT: u64 = 1 << 1;
    pub const REFERENCED: u64 = 1 << 2;
    pub const DIRTY: u64 = 1 << 3;
    pub const ACTIVE: u64 = 1 << 4;
    pub const UP_TO_DATE: u64 = 1 << 5;
    pub const DEVICE: u64 = 1 << 6;
    pub const KERNEL: u64 = 1 << 7;
    pub const KERNEL_SHARE_TO_USER: u64 = 1 << 8;
    pub const SLAB: u64 = 1 << 9;
}

/// BIOS 的 E820 条目恰好 20 字节。packed 字段不能直接借用给格式化器。
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct E820Entry {
    address: u64,
    length: u64,
    kind: u32,
}
impl E820Entry {
    const EMPTY: Self = Self {
        address: 0,
        length: 0,
        kind: 0,
    };
    fn end(self) -> usize {
        usize::try_from(
            self.address
                .checked_add(self.length)
                .expect("E820 address overflow"),
        )
        .expect("E820 address too large")
    }
    fn usable_range(self) -> Option<(usize, usize)> {
        if self.kind != 1 || self.length == 0 {
            return None;
        }
        // BIOS 描述的是字节区间，内核这一节管理的是 2 MiB 整页。两端向内取整，
        // 宁可暂不使用不足一页的 RAM，也不能把同一页中的 BIOS/设备保留字节分配出去。
        // 2 MiB 是本节页表使用的大页粒度，不意味着 x86-64 只能使用这种页大小。
        let start = align_up(self.address as usize, PAGE_2M_SIZE);
        let end = self.end() & !(PAGE_2M_SIZE - 1);
        (end > start).then_some((start, end))
    }
}
const _: () = assert!(size_of::<E820Entry>() == 20);

/// 与原实现一样，描述符按物理页号索引，空洞也占槽位，但永不标为空闲。
/// 位图解决“在哪里找空闲页”，这里解决“找到的页归谁、是否仍被使用”。将两者
/// 分开，既保持紧凑的搜索信息，又能让释放操作核对页的身份，而不是只翻转一个位。
#[repr(C)]
pub struct Page {
    zone: *mut Zone,
    physical_address: usize,
    attribute: u64,
    reference_count: usize,
    create_time: u64,
}
#[repr(C)]
pub struct Zone {
    pages: *mut Page,
    number_pages: usize,
    start_address: usize,
    end_address: usize,
    attribute: u64,
    length: usize,
    descriptor: *mut MemoryDescriptor,
    page_using_count: usize,
    page_free_count: usize,
    total_pages_ref_count: usize,
}

pub struct MemoryDescriptor {
    e820: [E820Entry; MAX_ENTRIES],
    number_entries: usize,
    bits_map: *mut u64,
    bits_size: usize,
    bits_length: usize,
    pages: *mut Page,
    pages_length: usize,
    zones: *mut Zone,
    zones_size: usize,
    end_of_struct: usize,
    global_cr3: u64,
}
static mut MEMORY: MemoryDescriptor = MemoryDescriptor {
    e820: [E820Entry::EMPTY; MAX_ENTRIES],
    number_entries: 0,
    bits_map: null_mut(),
    bits_size: 0,
    bits_length: 0,
    pages: null_mut(),
    pages_length: 0,
    zones: null_mut(),
    zones_size: 0,
    end_of_struct: 0,
    global_cr3: 0,
};
unsafe extern "C" {
    static _text: u8;
    static _endtext: u8;
    static _enddata: u8;
    static _end: u8;
}

pub fn align_up(address: usize, alignment: usize) -> usize {
    assert!(alignment.is_power_of_two());
    address
        .checked_add(alignment - 1)
        .expect("alignment overflow")
        & !(alignment - 1)
}

impl MemoryDescriptor {
    /// 1 表示已用/不可分配，0 表示空闲。必须写回，不能只修改局部副本。
    unsafe fn set_used(&mut self, index: usize, used: bool) {
        assert!(index < self.bits_size);
        // SAFETY: 位图已分配 ceil(bits_size/64) 个 u64，index 已限界。
        unsafe {
            let word = self.bits_map.add(index / 64);
            let mask = 1u64 << (index % 64);
            word.write(if used {
                word.read() | mask
            } else {
                word.read() & !mask
            });
        }
    }

    /// 保留原 init_page 的共享引用语义；页 0/空洞无 Zone，不参与 Zone 计数。
    unsafe fn init_page(&mut self, index: usize, attributes: u64) {
        assert!(index < self.bits_size);
        // SAFETY: Page/Zone 数组已初始化，单核串行操作，没有外部别名引用。
        unsafe {
            let page = &mut *self.pages.add(index);
            if page.attribute == 0 {
                page.attribute = attributes;
                page.reference_count = 1;
                if let Some(zone) = page.zone.as_mut() {
                    assert!(zone.page_free_count > 0);
                    zone.page_using_count += 1;
                    zone.page_free_count -= 1;
                    zone.total_pages_ref_count += 1;
                }
            } else if (page.attribute | attributes)
                & (flags::REFERENCED | flags::KERNEL_SHARE_TO_USER)
                != 0
            {
                page.attribute |= attributes;
                page.reference_count += 1;
                if let Some(zone) = page.zone.as_mut() {
                    zone.total_pages_ref_count += 1;
                }
            } else {
                page.attribute |= attributes;
            }
            self.set_used(index, true);
        }
    }

    /// 分配器归还引用；预留内核页及无 Zone 的物理空洞不能被释放。
    unsafe fn clean_page(&mut self, index: usize) {
        assert!(index < self.bits_size);
        // SAFETY: 调用者独占描述符，Page/Zone 指针来自本模块已验证的元数据区。
        unsafe {
            let page = &mut *self.pages.add(index);
            assert!(!page.zone.is_null() && page.attribute & flags::KERNEL_INIT == 0);
            if page.attribute == 0 {
                return;
            }
            assert!(page.reference_count > 0);
            let zone = &mut *page.zone;
            page.reference_count -= 1;
            zone.total_pages_ref_count -= 1;
            if page.reference_count == 0 {
                page.attribute = 0;
                zone.page_using_count -= 1;
                zone.page_free_count += 1;
                self.set_used(index, false);
            }
        }
    }
}

pub unsafe fn init() {
    // SAFETY: 仅 Start_Kernel 调用一次，中断关闭；运行期间无其他访问 MEMORY 的执行流。
    unsafe {
        let memory = &mut *addr_of_mut!(MEMORY);
        let count = read_unaligned((PAGE_OFFSET + 0x4ff0) as *const u16) as usize;
        assert!(count > 0 && count <= MAX_ENTRIES, "invalid E820 count");
        memory.number_entries = count;
        let mut physical_end = 0;
        let mut total_ram = 0u64;
        let mut total_pages = 0;
        kprint!(
            console::BLUE,
            "Physical address map (1=RAM, 2=reserved, 3=ACPI reclaim, 4=NVS, 5=bad)\n"
        );
        for i in 0..count {
            let entry = read_unaligned((PAGE_OFFSET + 0x5000 + i * 20) as *const E820Entry);
            memory.e820[i] = entry;
            let E820Entry {
                address,
                length,
                kind,
            } = entry;
            kprint!(
                console::ORANGE,
                "Address:{address:#018x} Length:{length:#018x} Type:{kind}\n"
            );
            // 不能假定 BIOS 按地址排序，不能把 type=5 当成终止标记。
            physical_end = physical_end.max(entry.end());
            if kind == 1 {
                total_ram = total_ram.checked_add(length).expect("RAM sum overflow");
            }
            if let Some((start, end)) = entry.usable_range() {
                total_pages += (end - start) / PAGE_2M_SIZE;
            }
        }
        assert!(total_pages > 0, "no usable 2 MiB pages");
        kprint!(
            console::ORANGE,
            "OS usable RAM:{total_ram:#018x}; usable 2M pages:{total_pages}\n"
        );
        memory.bits_size = align_up(physical_end, PAGE_2M_SIZE) / PAGE_2M_SIZE;
        memory.bits_length = memory
            .bits_size
            .div_ceil(64)
            .checked_mul(8)
            .expect("bitmap overflow");
        memory.pages_length = memory
            .bits_size
            .checked_mul(size_of::<Page>())
            .expect("Page array overflow");
        let kernel_end = addr_of!(_end) as usize;
        let bitmap = align_up(kernel_end, PAGE_4K_SIZE);
        let pages = align_up(
            bitmap
                .checked_add(memory.bits_length)
                .expect("bitmap end overflow"),
            PAGE_4K_SIZE,
        );
        let zones = align_up(
            pages
                .checked_add(memory.pages_length)
                .expect("Page end overflow"),
            PAGE_4K_SIZE,
        );
        let metadata_end = align_up(
            zones
                .checked_add(MAX_ZONES * size_of::<Zone>())
                .expect("Zone end overflow"),
            256,
        );
        let physical_metadata_end = metadata_end - PAGE_OFFSET;
        // 地址转换仅适用于 head.S 的直映 RAM，而不是整个 64 位物理地址空间。
        assert!(
            physical_metadata_end <= DIRECT_RAM_END,
            "metadata exceeds initial 10 MiB RAM mapping"
        );
        assert!(
            memory.e820[..count].iter().any(|e| e.kind == 1
                && e.address <= (kernel_end - PAGE_OFFSET) as u64
                && e.end() >= physical_metadata_end),
            "metadata is not contained in usable RAM"
        );
        memory.bits_map = bitmap as *mut u64;
        memory.pages = pages as *mut Page;
        memory.zones = zones as *mut Zone;
        memory.end_of_struct = metadata_end;
        // 到这里才开始写元数据，避免先越界写入再检查。
        core::ptr::write_bytes(memory.bits_map.cast::<u8>(), 0xff, memory.bits_length);
        core::ptr::write_bytes(memory.pages.cast::<u8>(), 0, memory.pages_length);
        core::ptr::write_bytes(memory.zones.cast::<u8>(), 0, MAX_ZONES * size_of::<Zone>());
        for i in 0..memory.bits_size {
            (*memory.pages.add(i)).physical_address = i * PAGE_2M_SIZE;
        }
        for i in 0..count {
            let Some((start, end)) = memory.e820[i].usable_range() else {
                continue;
            };
            assert!(memory.zones_size < MAX_ZONES);
            let zone = memory.zones.add(memory.zones_size);
            memory.zones_size += 1;
            let first = start / PAGE_2M_SIZE;
            let number = (end - start) / PAGE_2M_SIZE;
            zone.write(Zone {
                pages: memory.pages.add(first),
                number_pages: number,
                start_address: start,
                end_address: end,
                attribute: 0,
                length: end - start,
                descriptor: memory,
                page_using_count: 0,
                page_free_count: number,
                total_pages_ref_count: 0,
            });
            for j in first..first + number {
                assert!(
                    (*memory.pages.add(j)).zone.is_null(),
                    "overlapping E820 RAM zones"
                );
                (*memory.pages.add(j)).zone = zone;
                memory.set_used(j, false);
            }
        }
        // 整个 [0, end_of_struct) 都保留，包括 BIOS、Loader、内核、栈及元数据。
        // 页描述符和位图本身也住在 RAM 中：若忘记保护“管理内存所用的内存”，
        // 分配器就会把自己的账本交给调用者覆盖。末端即使只占了半页，也必须整页保留。
        // 页 0 是混合区域，不能假装属于从 2 MiB 开始的第一个 Zone。
        let reserved_pages = physical_metadata_end.div_ceil(PAGE_2M_SIZE);
        for index in 0..reserved_pages {
            memory.init_page(
                index,
                flags::PAGE_TABLE_MAPPED | flags::KERNEL_INIT | flags::ACTIVE | flags::KERNEL,
            );
        }
        // 实现完成后的运行时一致性检查：位图、引用数和 Zone 计数必须相符。
        let mut free_pages = 0;
        for i in 0..memory.zones_size {
            let zone = &*memory.zones.add(i);
            let mut free = 0;
            let mut references = 0;
            for j in zone.start_address / PAGE_2M_SIZE..zone.end_address / PAGE_2M_SIZE {
                let used = memory.bits_map.add(j / 64).read() & (1 << (j % 64)) != 0;
                let page = &*memory.pages.add(j);
                assert_eq!(used, page.attribute != 0);
                if !used {
                    free += 1;
                }
                references += page.reference_count;
            }
            assert_eq!(free, zone.page_free_count);
            assert_eq!(zone.page_using_count + free, zone.number_pages);
            assert_eq!(references, zone.total_pages_ref_count);
            free_pages += free;
            kprint!(
                console::ORANGE,
                "Zone {i}: {:#018x}..{:#018x}, pages={} free={} used={} refs={}\n",
                zone.start_address,
                zone.end_address,
                zone.number_pages,
                free,
                zone.page_using_count,
                references
            );
        }
        kprint!(
            console::ORANGE,
            "bits_map:{bitmap:#018x} bits_size:{} bits_length:{}\n",
            memory.bits_size,
            memory.bits_length
        );
        kprint!(
            console::ORANGE,
            "pages:{pages:#018x} pages_length:{} zones:{zones:#018x} zones_count:{}\n",
            memory.pages_length,
            memory.zones_size
        );
        kprint!(
            console::ORANGE,
            "start_code:{:#018x} end_code:{:#018x} end_data:{:#018x} end_brk:{kernel_end:#018x} end_of_struct:{metadata_end:#018x}\n",
            addr_of!(_text) as usize,
            addr_of!(_endtext) as usize,
            addr_of!(_enddata) as usize
        );
        kprint!(
            console::GREEN,
            "Memory invariants OK: free={free_pages} reserved={reserved_pages}\n"
        );
        memory.global_cr3 = arch::cr3();
        let pml4 = (PAGE_OFFSET + memory.global_cr3 as usize) as *mut u64;
        let pdpt_physical = pml4.read() & 0x000f_ffff_ffff_f000;
        let pdpt = (PAGE_OFFSET + pdpt_physical as usize) as *const u64;
        kprint!(
            console::INDIGO,
            "Global_CR3:{:#018x} PML4[0]:{:#018x}\n",
            memory.global_cr3,
            pml4.read()
        );
        kprint!(console::PURPLE, "PDPT[0]:{:#018x}\n", pdpt.read());
        // PML4[256] 保留高半区映射；只清除 [0] 的低地址别名，然后刷新 TLB。
        // 低地址映射最初是为模式切换准备的：CPU 仍在低地址取指，不能提前撤掉。
        // 现在代码/栈/数据都已搬到高半区，就不必再让同一批物理内存占据两套地址。
        // 这也让空指针等低地址误用及时触发 #PF，并把低虚拟空间留给未来的用户程序。
        // 不是“永远不能映射低地址”：5-1 会按用户程序需要重新建立独立、有限的映射，
        // 而不恢复包含 BIOS/内核/显存的整片恒等别名。用户隔离还依赖各级 U/S 权限。
        pml4.write(0);
        arch::flush_tlb();
        assert_eq!(pml4.read(), 0);
        assert_ne!(pml4.add(256).read(), 0);
        kprint!(
            console::GREEN,
            "Identity mapping removed; high-half mapping retained\n"
        );
    }
}
