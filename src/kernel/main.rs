//! 第 4 章内核主程序：图形输出 → 异常向量 → 初级物理内存描述。
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
