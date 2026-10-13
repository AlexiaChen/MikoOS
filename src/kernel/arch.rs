//! CPU 特权指令的薄封装。这里显式保留指令，便于对照教材和反汇编。
use core::arch::asm;

/// debugcon 是模拟器调试端口；真实硬件上不将它作为设备驱动使用。
pub fn debug_byte(byte: u8) {
    // SAFETY: 当前运行在 ring 0；0xe9 仅用于约定的模拟器调试输出。
    unsafe {
        asm!("out dx, al", in("dx") 0xe9u16, in("al") byte, options(nomem, nostack, preserves_flags))
    }
}

#[unsafe(export_name = "kernel_halt")]
#[inline(never)]
pub fn halt() -> ! {
    loop {
        // SAFETY: 本节无外部中断驱动，关中断后停机；NMI 唤醒时继续停机。
        unsafe { asm!("cli", "hlt", options(nomem, nostack)) }
    }
}

pub fn cr2() -> u64 {
    let value;
    // SAFETY: ring 0 只读故障线性地址，不改变 CPU 状态。
    unsafe { asm!("mov {}, cr2", out(reg) value, options(nomem, nostack, preserves_flags)) }
    value
}

pub fn cr3() -> u64 {
    let value;
    // SAFETY: ring 0 只读当前页表物理基址。
    unsafe { asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags)) }
    value
}

/// 修改页表后重载 CR3。不能标注 nomem，否则编译器可能重排页表写入。
pub unsafe fn flush_tlb() {
    // SAFETY: 调用者保证当前代码、栈及数据在修改后的页表里仍可访问。
    unsafe { asm!("mov {0}, cr3", "mov cr3, {0}", out(reg) _, options(nostack, preserves_flags)) }
}

/// 单页失效接口，留给后续逐页映射章节。
#[allow(dead_code)]
pub unsafe fn flush_tlb_one(address: u64) {
    // SAFETY: ring 0 执行；操作只失效 TLB，不解引用 address。
    unsafe { asm!("invlpg [{}]", in(reg) address, options(nostack, preserves_flags)) }
}

// LLVM 可能把 Rust 的内存操作降低成这些裸机符号。用 rep 指令实现，
// 避免 Rust 循环再次被优化成 memcpy/memset 造成递归，不依赖 libc。
#[unsafe(no_mangle)]
unsafe extern "C" fn memset(dst: *mut u8, byte: i32, len: usize) -> *mut u8 {
    // SAFETY: C ABI 调用者保证 dst 的 len 字节可写；DF 由入口清零。
    unsafe {
        asm!("rep stosb", inout("rdi") dst => _, in("al") byte as u8, inout("rcx") len => _, options(nostack))
    }
    dst
}

#[unsafe(no_mangle)]
unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    // SAFETY: C ABI 调用者保证范围有效且不重叠，DF=0。
    unsafe {
        asm!("rep movsb", inout("rdi") dst => _, inout("rsi") src => _, inout("rcx") len => _, options(nostack))
    }
    dst
}

#[unsafe(no_mangle)]
unsafe extern "C" fn memmove(dst: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    if dst.addr() > src.addr() && dst.addr() - src.addr() < len {
        // SAFETY: 有重叠且目标较高时从尾部复制；退出前恢复 DF=0 的 ABI 约束。
        unsafe {
            asm!("std", "rep movsb", "cld", inout("rdi") dst.add(len - 1) => _, inout("rsi") src.add(len - 1) => _, inout("rcx") len => _, options(nostack))
        }
        dst
    } else {
        // SAFETY: 此分支向前复制不会覆盖未读的源；len=0 也可执行 rep。
        unsafe { memcpy(dst, src, len) }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, len: usize) -> i32 {
    for i in 0..len {
        // SAFETY: C ABI 调用者保证两个范围的 len 字节均可读。
        let (x, y) = unsafe { (a.add(i).read(), b.add(i).read()) };
        if x != y {
            return i32::from(x) - i32::from(y);
        }
    }
    0
}
