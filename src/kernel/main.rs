//! 第 4 章内核主程序：图形输出 → 异常向量 → 物理页描述与分配。
#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

mod arch;
mod console;
mod font;
mod gate;
mod memory;
mod trap;

use core::{arch::global_asm, panic::PanicInfo};

// 在 Rust 构建中直接汇编边界代码，不再需要 C 预处理器或 GCC。
global_asm!(include_str!("head.S"), options(att_syntax));
global_asm!(include_str!("entry.S"), options(att_syntax));

#[cfg(all(feature = "fault-divide", feature = "fault-page"))]
compile_error!("一次只启用一个异常演示功能");

#[unsafe(no_mangle)]
extern "C" fn Start_Kernel() -> ! {
    console::init();
    kprint!(console::YELLOW, "Hello\tMiko\tOS!\n");
    // SAFETY: head.S 已清零 BSS 并进入高半区；此时中断关闭，单核初始化。
    unsafe {
        gate::init();
        trap::init();
    }
    kprint!(console::GREEN, "GDT/IDT/TSS ready\n");
    kprint!(console::RED, "init memory\n");
    // SAFETY: Loader 已交接 E820，链接器定义的 _end 之后是初始直映 RAM。
    unsafe { memory::init() };
    // 分配得到的是物理页所有权，不是一个 Rust 引用。这里先各取一页，确认
    // 两种区域选择不会把同一页重复分配，再归还；正常启动不保留演示用的 RAM。
    // SAFETY: 内存初始化完成，仍为单核关中断；期间没有其他内存分配使用者。
    unsafe {
        let mapped = memory::allocate_pages(memory::AllocationArea::Mapped, 1)
            .expect("no free bootstrap-mapped page");
        let physical =
            memory::allocate_pages(memory::AllocationArea::Any, 1).expect("no free physical page");
        assert_ne!(mapped.physical_address(), physical.physical_address());
        // CPU 已能通过高半区直映访问 mapped；只对明确映射的独占页做实际读写。
        // volatile 保证访问真的发生，避免编译器用常量替代，掩盖页表地址错误。
        let first = (memory::PAGE_OFFSET + mapped.physical_address()) as *mut u64;
        let last = first.byte_add(mapped.page_count() * memory::PAGE_2M_SIZE - 8);
        first.write_volatile(0x4d49_4b4f_4f53_0001);
        last.write_volatile(0x4d49_4b4f_4f53_0002);
        assert_eq!(first.read_volatile(), 0x4d49_4b4f_4f53_0001);
        assert_eq!(last.read_volatile(), 0x4d49_4b4f_4f53_0002);
        memory::free_pages(physical);
        memory::free_pages(mapped);
    }
    kprint!(
        console::GREEN,
        "Physical page allocation/read-write/free OK\n"
    );
    #[cfg(feature = "memory-check")]
    // SAFETY: 同上；扩展检查只修改分配元数据，并在完成时归还全部页。
    unsafe {
        memory::verify()
    };
    kprint!(console::GREEN, "MikoOS Rust kernel ready\n");

    #[cfg(feature = "fault-divide")]
    // SAFETY: 显式教学演示 CPU #DE；不用 Rust 的整数除零（那会 panic）。
    unsafe {
        core::arch::asm!("xor edx, edx", "mov eax, 1", "xor ecx, ecx", "div rcx", out("rax") _, out("rcx") _, out("rdx") _, options(nostack));
    }
    #[cfg(feature = "fault-page")]
    // SAFETY: 显式教学演示 #PF，访问当前 PDE 未映射的规范高半区地址。
    unsafe {
        core::arch::asm!("mov rax, [rax]", inout("rax") 0xffff_8000_0aa0_0000u64 => _, options(nostack));
    }
    arch::halt()
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    kprint!(console::RED, "PANIC: {info}\n");
    arch::halt()
}
