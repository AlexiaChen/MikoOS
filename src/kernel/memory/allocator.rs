//! 连续物理页：把“知道哪些 RAM 空闲”推进到“把一段 RAM 交给调用者”。
//!
//! CPU 的 load/store 使用线性地址，分页硬件再把它翻译成物理地址。本模块只负责
//! 物理页的所有权记账，不修改页表。因此返回一个物理地址并不意味着 Rust 可以
//! 解引用它：当前启动页表只直映前 10 MiB，再往后的虚拟窗口还用于帧缓冲。
//! 先分清“分配”和“映射”，后续扩展页表时才不会把显存误当普通 RAM 写坏。
use super::*;

/// 按真实的页表覆盖范围选页，不套用教材中尚未完善的 DMA/NORMAL Zone 索引。
/// E820 的 Zone 表达 RAM 的物理分段，不能用 Zone 的序号推断 CPU 能否访问它。
#[derive(Clone, Copy)]
pub enum AllocationArea {
    /// 整段位于启动页表的 RAM 直映区，可以用 PAGE_OFFSET + 物理地址访问。
    Mapped,
    /// 任意完整 RAM 页。调用者使用前必须另行确认映射，不能直接加偏移解引用。
    Any,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AllocationError {
    InvalidCount,
    OutOfMemory,
}

/// 一段连续物理页的独占凭据。字段私有，且不实现 Copy/Clone：
/// 同一凭据只能交还一次，避免把旧的页号再次释放、误伤后来的使用者。
/// 当前采用显式 free_pages，而不是 Drop，便于看清物理页生命周期；丢弃凭据会泄漏。
#[must_use = "物理页用完后必须交给 free_pages"]
#[derive(Debug)]
pub struct PageRun {
    first: usize,
    count: usize,
}

impl PageRun {
    pub fn physical_address(&self) -> usize {
        self.first * PAGE_2M_SIZE
    }

    pub fn page_count(&self) -> usize {
        self.count
    }
}

/// 首次适配：在单个 Zone 内寻找连续的空闲位，找到整段之后才提交占用。
///
/// # Safety
/// memory::init 已完成；调用期间独占全局内存描述符。目前仅适用于启动阶段的
/// 单核、关中断环境。unsafe 表达的是这个并发前提，不是“地址随便可以访问”。
pub unsafe fn allocate_pages(
    area: AllocationArea,
    count: usize,
) -> Result<PageRun, AllocationError> {
    // SAFETY: 调用者保证初始化完成且无中断/其他 CPU 并发访问 MEMORY。
    unsafe { (&mut *addr_of_mut!(MEMORY)).allocate(area, count) }
}

/// 归还整段独占物理页，消费凭据；这不执行解除映射或 TLB 失效。
///
/// # Safety
/// 与 allocate_pages 一样必须独占描述符；调用者已停止访问这些页，并处理完
/// 指向它们的映射或其他使用者。否则页交给新主人后，旧指针仍可能破坏新数据。
pub unsafe fn free_pages(run: PageRun) {
    // SAFETY: 调用者遵守独占访问和停止使用约定，凭据只能由本模块创建。
    unsafe { (&mut *addr_of_mut!(MEMORY)).release(run) }
}

impl MemoryDescriptor {
    unsafe fn is_used(&self, index: usize) -> bool {
        assert!(index < self.bits_size);
        // SAFETY: 已初始化的位图容纳 ceil(bits_size/64) 个 u64，索引已经检查。
        unsafe { self.bits_map.add(index / 64).read() & (1u64 << (index % 64)) != 0 }
    }

    unsafe fn allocate(
        &mut self,
        area: AllocationArea,
        count: usize,
    ) -> Result<PageRun, AllocationError> {
        if count == 0 {
            return Err(AllocationError::InvalidCount);
        }
        // SAFETY: self 是唯一可变借用，元数据在 init 时已验证并完全初始化。
        unsafe {
            for i in 0..self.zones_size {
                // 复制边界值后结束对 Zone 的借用，再通过 self 更新记账。
                let zone = &*self.zones.add(i);
                let start = zone.start_address / PAGE_2M_SIZE;
                let end_address = match area {
                    AllocationArea::Mapped => zone.end_address.min(DIRECT_RAM_END),
                    AllocationArea::Any => zone.end_address,
                };
                let end = end_address / PAGE_2M_SIZE;
                if count > end.saturating_sub(start) || count > zone.page_free_count {
                    continue;
                }

                // 位图压缩“是否占用”，Page/Zone 则保存归属、属性及计数，两者用途不同。
                // 顺序逐位扫描虽不是最快，但不会出现原版跨 u64 读取 p+1 越界、
                // 移位 64 位或 page=j+k-1 的错位问题，也自然支持超过 64 页的请求。
                // 每进入一个 Zone 都重置连续长度，不能跨 BIOS 保留区或 RAM 空洞拼页。
                let mut consecutive = 0;
                for index in start..end {
                    if self.is_used(index) {
                        consecutive = 0;
                        continue;
                    }
                    consecutive += 1;
                    if consecutive != count {
                        continue;
                    }
                    let first = index + 1 - count;
                    // 到此才修改状态：失败的搜索不能留下一半占用的页或错误的计数。
                    for page in first..=index {
                        let mut attributes = flags::ACTIVE | flags::KERNEL;
                        if page < DIRECT_RAM_END / PAGE_2M_SIZE {
                            attributes |= flags::PAGE_TABLE_MAPPED;
                        }
                        self.init_page(page, attributes);
                    }
                    return Ok(PageRun { first, count });
                }
            }
        }
        Err(AllocationError::OutOfMemory)
    }

    unsafe fn release(&mut self, run: PageRun) {
        let end = run
            .first
            .checked_add(run.count)
            .expect("page range overflow");
        assert!(run.count > 0 && end <= self.bits_size);
        // SAFETY: 凭据来自本分配器，调用者已停止使用；所有索引均在 Page 数组内。
        unsafe {
            let zone = (*self.pages.add(run.first)).zone;
            assert!(!zone.is_null());
            // 先验证整段再释放，不能释放到一半才发现页属于内核初始化区域。
            // 当前凭据代表独占页；共享引用留给之后的进程地址空间章节。
            for index in run.first..end {
                let page = &*self.pages.add(index);
                assert_eq!(page.zone, zone);
                assert!(self.is_used(index));
                assert!(page.attribute != 0 && page.attribute & flags::KERNEL_INIT == 0);
                assert_eq!(page.reference_count, 1);
            }
            for index in run.first..end {
                self.clean_page(index);
            }
        }
    }
}

/// 实现完成后的验证，只有显式启用 memory-check 才执行，不是正常启动的工作负载。
/// 检查分配记账，不读取尚未映射的物理 RAM；高地址测试也不会悄悄扩展页表。
///
/// # Safety
/// memory::init 已完成，单核关中断，独占全局描述符；没有其他物理页使用者。
#[cfg(feature = "memory-check")]
pub unsafe fn verify() {
    // SAFETY: Start_Kernel 在串行初始化结束后调用，未启用任何外部中断。
    unsafe {
        let memory = &mut *addr_of_mut!(MEMORY);
        let baseline = memory.check_allocation_state();
        assert_eq!(
            memory.allocate(AllocationArea::Any, 0).unwrap_err(),
            AllocationError::InvalidCount
        );
        assert_eq!(
            memory
                .allocate(AllocationArea::Any, usize::MAX)
                .unwrap_err(),
            AllocationError::OutOfMemory
        );
        assert_eq!(memory.check_allocation_state(), baseline);

        // 默认 2 GiB 机器第一段 RAM 从页号 1 开始。65 页必定跨越 u64 位图边界；
        // 请求甚至超过一个字宽，验证算法不依赖 (1 << count) 这种有限宽度的掩码。
        let large = memory
            .allocate(AllocationArea::Any, 65)
            .expect("verification needs 65 contiguous pages");
        assert_eq!(large.page_count(), 65);
        assert_eq!(memory.check_allocation_state().1, baseline.1 - 65);
        let other = memory.allocate(AllocationArea::Any, 1).unwrap();
        assert!(other.first < large.first || other.first >= large.first + large.count);
        memory.check_allocation_state();
        let first = large.first;
        memory.release(large);
        memory.check_allocation_state();
        let reused = memory.allocate(AllocationArea::Any, 65).unwrap();
        assert_eq!(reused.first, first);
        memory.release(reused);
        memory.release(other);
        assert_eq!(memory.check_allocation_state(), baseline);

        // 把当前直映区逐页用完，再只还回一页制造碎片：总空闲数不等于连续空闲数。
        // 此验证最多需要 5 个凭据，与 10 MiB / 2 MiB 的启动映射契约一致。
        let mut held: [Option<PageRun>; DIRECT_RAM_END / PAGE_2M_SIZE] =
            core::array::from_fn(|_| None);
        let mut number = 0;
        while let Ok(run) = memory.allocate(AllocationArea::Mapped, 1) {
            assert!(run.physical_address() + PAGE_2M_SIZE <= DIRECT_RAM_END);
            held[number] = Some(run);
            number += 1;
        }
        assert!(number >= 2, "verification needs two mapped free pages");
        let hole = held[0].take().unwrap();
        let hole_first = hole.first;
        memory.release(hole);
        let fragmented = memory.check_allocation_state();
        assert_eq!(
            memory.allocate(AllocationArea::Mapped, 2).unwrap_err(),
            AllocationError::OutOfMemory
        );
        assert_eq!(memory.check_allocation_state(), fragmented);
        held[0] = Some(memory.allocate(AllocationArea::Mapped, 1).unwrap());
        assert_eq!(held[0].as_ref().unwrap().first, hole_first);
        for run in held.into_iter().flatten() {
            memory.release(run);
            memory.check_allocation_state();
        }
        assert_eq!(memory.check_allocation_state(), baseline);

        // 完整申请默认 Zone，覆盖最右侧页，检测最后一位的 off-by-one；耗尽后
        // 再请求一页应失败，不能从区域末尾继续搜索不可用的物理地址。
        // 这是针对仓库默认 Bochs 布局的验证，不假定任意机器都只有一段 RAM。
        // 其他布局明确停止，不把没有覆盖的验证打印为通过。
        assert_eq!(
            memory.zones_size, 1,
            "exhaustion verification requires the default single RAM zone"
        );
        assert_eq!(
            baseline.0, 0,
            "exhaustion verification requires an unreserved RAM zone"
        );
        let entire = memory.allocate(AllocationArea::Any, baseline.1).unwrap();
        let exhausted = memory.check_allocation_state();
        assert_eq!(exhausted.1, 0);
        assert_eq!(
            memory.allocate(AllocationArea::Any, 1).unwrap_err(),
            AllocationError::OutOfMemory
        );
        assert_eq!(memory.check_allocation_state(), exhausted);
        memory.release(entire);
        assert_eq!(memory.check_allocation_state(), baseline);
        kprint!(
            console::GREEN,
            "Page allocator checks OK: cross-word, reuse, fragmentation, exhaustion; free={}\n",
            baseline.1
        );
    }
}

#[cfg(feature = "memory-check")]
impl MemoryDescriptor {
    /// 把位图、页描述符、Zone 统计互相核对。只看空闲页总数会漏掉“一页被重复
    /// 分配、另一页漏记”这类总数恰好抵消的错误，因此逐页验证归属和引用数。
    unsafe fn check_allocation_state(&self) -> (usize, usize, usize) {
        // SAFETY: 只读已初始化且当前执行流独占的元数据；按数组容量限界。
        unsafe {
            let mut totals = (0, 0, 0);
            for i in 0..self.zones_size {
                let zone_ptr = self.zones.add(i);
                let zone = &*zone_ptr;
                let mut counts = (0, 0, 0);
                for index in zone.start_address / PAGE_2M_SIZE..zone.end_address / PAGE_2M_SIZE {
                    let page = &*self.pages.add(index);
                    assert_eq!(page.zone, zone_ptr);
                    assert_eq!(page.physical_address, index * PAGE_2M_SIZE);
                    assert_eq!(self.is_used(index), page.attribute != 0);
                    assert_eq!(page.attribute != 0, page.reference_count > 0);
                    if self.is_used(index) {
                        counts.0 += 1;
                    } else {
                        counts.1 += 1;
                    }
                    counts.2 += page.reference_count;
                }
                assert_eq!(
                    counts,
                    (
                        zone.page_using_count,
                        zone.page_free_count,
                        zone.total_pages_ref_count
                    )
                );
                assert_eq!(counts.0 + counts.1, zone.number_pages);
                totals.0 += counts.0;
                totals.1 += counts.1;
                totals.2 += counts.2;
            }
            for index in 0..self.bits_size {
                let page = &*self.pages.add(index);
                if page.zone.is_null() || page.attribute & flags::KERNEL_INIT != 0 {
                    assert!(self.is_used(index), "reserved page became free");
                }
            }
            totals
        }
    }
}
