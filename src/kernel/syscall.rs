//! 第 5 章 5-2/5-3：跨特权级的请求协议与第一个服务。
//!
//! SYSENTER 只把 CPU 带到预设的内核 RIP/RSP，并不保存用户返回现场。
//! 因此它不是普通 CALL，也不是中断门：RCX/RDX 的含义必须由两侧约定，
//! 服务号、用户指针、返回地址都必须视为输入，不能因为已经处于 CPL0 就信任它们。
use crate::{arch, console, kprint, usermode};
use core::{arch::asm, ptr};

pub const WRITE: u64 = 1;
pub const MAX_WRITE: usize = 256;
pub const ENOSYS: i64 = -38;
pub const EFAULT: i64 = -14;
pub const EINVAL: i64 = -22;

/// 与 SAVE_REGISTERS 的压栈顺序一致，只有 GPR，没有伪造的 CPU 中断现场。
/// RCX/RDX 是用户主动提供的返回 RSP/RIP，并非 CPU 保存的可信值。
#[repr(C)]
pub struct Frame {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rsi: u64,
    rdi: u64,
    rbp: u64,
    pub rdx: u64,
    pub rcx: u64,
    rbx: u64,
    rax: u64,
}
const _: () = assert!(core::mem::offset_of!(Frame, rax) == 112);
const _: () = assert!(core::mem::offset_of!(Frame, rcx) == 96);
const _: () = assert!(core::mem::offset_of!(Frame, rdx) == 88);
const _: () = assert!(core::mem::offset_of!(Frame, rdi) == 72);
const _: () = assert!(core::mem::offset_of!(Frame, rsi) == 64);
const _: () = assert!(core::mem::size_of::<Frame>() == 120);

type Service = fn(&Frame) -> i64;
// 保留教材的 128 槽分发表；默认槽也必须有定义，不能调用未初始化函数指针。
static SERVICES: [Service; 128] = {
    let mut table = [unimplemented as Service; 128];
    table[WRITE as usize] = write_text;
    table
};

unsafe extern "C" {
    fn system_call_entry();
}

/// # Safety
/// Intel64/SEP 已核对，单核 IF=0；stack_top 指向存活、16 字节对齐的独立内核栈。
pub unsafe fn enable(stack_top: usize) {
    assert!(!arch::interrupts_enabled());
    assert_eq!(stack_top % 16, 0);
    // SAFETY: CPL0 可访问这三个 MSR；先准备栈和入口，最后设置非零 CS 启用协议。
    // Intel64 的 ESP/EIP MSR 实际容纳完整 64 位地址，不能截断高半区栈/入口。
    unsafe {
        write_msr(0x175, stack_top as u64);
        write_msr(0x176, system_call_entry as *const () as u64);
        write_msr(0x174, 8);
    }
    kprint!(
        console::GREEN,
        "SYSENTER ready: CS=0x8, dedicated kernel stack, 128 services\n"
    );
}

/// # Safety
/// 单核 IF=0，用户演示已结束，不再需要 SYSEXIT；没有活动系统调用。
pub unsafe fn disable() {
    // SAFETY: 先清 CS；之后若误执行 SYSENTER，CPU 因空选择子产生 #GP，
    // 而不是跳入已撤销的服务环境。再清入口/栈，不留下悬空硬件配置。
    unsafe {
        write_msr(0x174, 0);
        write_msr(0x175, 0);
        write_msr(0x176, 0);
    }
}

unsafe fn write_msr(index: u32, value: u64) {
    let low: u32;
    let high: u32;
    // SAFETY: 仅由上面的 CPL0 初始化/关闭流程调用，index 为已知 SYSENTER MSR。
    unsafe {
        asm!("wrmsr", in("ecx") index, in("eax") value as u32,
            in("edx") (value >> 32) as u32, options(nostack));
        asm!("rdmsr", in("ecx") index, out("eax") low,
            out("edx") high, options(nostack));
    }
    assert_eq!(u64::from(low) | (u64::from(high) << 32), value);
}

#[unsafe(no_mangle)]
extern "C" fn system_call_dispatch(frame: &Frame) -> i64 {
    assert!(!arch::interrupts_enabled());
    // 先检查入口栈及返回目标。SYSENTER 没有保存 CS/RFLAGS，不能声称这里
    // 有硬件保存的 CPL3 现场；演示最后的 INT 0x80 另行核对真实用户现场。
    if !usermode::valid_syscall(frame) {
        kprint!(console::RED, "SYSENTER rejected invalid return context\n");
        arch::halt();
    }
    // 用户可传任意 u64，包括 128 和 u64::MAX。get 检查边界，禁止直接以
    // 用户调用号索引内核函数指针；范围内的空槽与范围外统一返回 -ENOSYS。
    SERVICES
        .get(frame.rax as usize)
        .copied()
        .unwrap_or(unimplemented)(frame)
}

fn unimplemented(_: &Frame) -> i64 {
    ENOSYS
}

fn write_text(frame: &Frame) -> i64 {
    let length = frame.rsi;
    if length > MAX_WRITE as u64 {
        return EINVAL;
    }
    // 零长度不读指针，允许 write(NULL, 0)。非空请求须整个区间都可读；
    // “地址低于内核”不足以证明已映射，末地址的整数溢出也必须拒绝。
    if length == 0 {
        return 0;
    }
    if !usermode::readable(frame.rdi, length) {
        return EFAULT;
    }
    let mut copy = [0u8; MAX_WRITE];
    for (offset, byte) in copy[..length as usize].iter_mut().enumerate() {
        // SAFETY: readable 核对当前两页布局和完整区间；单核 IF=0，映射/分配
        // 在本次服务期间不变，用户也不能并发改写。不把用户内存借成 Rust 引用。
        // 将来有并发撤映射/缺页时，须改成可恢复 fault 的 copy_from_user。
        *byte = unsafe { ptr::read_volatile((frame.rdi as *const u8).add(offset)) };
    }
    let Ok(text) = core::str::from_utf8(&copy[..length as usize]) else {
        return EINVAL;
    };
    // 用户数据只作为参数，绝不当作格式串；%s/%n/花括号等均按原文输出。
    // 复制完成并核对 UTF-8 后才产生输出，错误请求不会输出半截数据。
    kprint!(console::GREEN, "{text}");
    length as i64
}

/// 用户侧薄包装，必须内联进可复制载荷；不能产生到高半区内核函数的 CALL。
/// RAX=号/返回值，RDI/RSI=参数，RCX=返回 RSP，RDX=返回 RIP。
/// 内核保留其余 GPR，算术标志不保留；本节返回 IF=IOPL=DF=0。
#[inline(always)]
#[cfg(not(any(
    feature = "fault-user-kernel",
    feature = "fault-user-write",
    feature = "fault-user-exec",
    feature = "fault-user-privileged"
)))]
pub fn request(number: u64, first: u64, second: u64) -> i64 {
    let result: i64;
    // SAFETY: 仅在演示用户映射及 SYSENTER MSR 均有效时调用；局部标签随载荷
    // 同址搬迁。不能标 nomem/readonly：服务会读取用户缓冲并产生可见输出。
    unsafe {
        asm!("lea rdx, [rip + 2f]", "mov rcx, rsp", "sysenter", "2:",
            inlateout("rax") number => result, in("rdi") first, in("rsi") second,
            lateout("rcx") _, lateout("rdx") _, options(nostack));
    }
    result
}
