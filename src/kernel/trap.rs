//! 第 4 章系统异常：入口汇编只负责规范化现场，解释与输出均在 Rust 中完成。
use crate::{
    arch, console,
    gate::{Gate, TablePointer},
    kprint,
};
use core::arch::asm;

static mut IDT: [Gate; 256] = [Gate::EMPTY; 256];
unsafe extern "C" {
    static exception_entries: [u64; 32];
    fn exception_255();
}

/// 与 entry.S 的压栈顺序严格一致。IST 切栈使 CPU 保存旧 RSP/SS。
#[repr(C)]
pub struct InterruptFrame {
    pub registers: [u64; 15], // r15..r8, rsi,rdi,rbp,rdx,rcx,rbx,rax
    pub vector: u64,
    pub error: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}
const _: () = assert!(core::mem::offset_of!(InterruptFrame, rip) == 136);
const _: () = assert!(core::mem::size_of::<InterruptFrame>() == 176);

/// 本节完成通知门：DPL3 允许用户执行 INT 0x80；IST=0 让真实的 CPL3→0
/// 转换从 TSS.RSP0 选择栈。演示结束必须关闭，不能留下通用系统调用入口。
///
/// # Safety
/// 单核 IF=0，TSS.RSP0 已指向独立、存活的完成通知栈，handler 有完整 CPU 入口协议。
pub unsafe fn set_user_return_gate(handler: Option<u64>) {
    // SAFETY: 启动阶段独占 IDT，仅更新保留的 0x80 门，结束时恢复原 fatal 门。
    unsafe {
        let gate = match handler {
            Some(address) => Gate::new(address, 0xee, 0),
            None => Gate::new(exception_255 as *const () as u64, 0x8f, 1),
        };
        core::ptr::addr_of_mut!(IDT)
            .cast::<Gate>()
            .add(0x80)
            .write(gate);
    }
}

/// 在现有 IDT 中安装 PIC 的 16 个中断门。IDTR 指向的是这块内存，更新门无需
/// 再 LIDT；但必须在 IF=0 时完成，不能让 CPU 看见只写了一半的 16 字节门。
///
/// # Safety
/// 单核、关中断，TSS/IST3 已初始化；handler 是保存完整现场并以 IRETQ 返回的入口。
pub unsafe fn install_irq(vector: usize, handler: u64) {
    assert!((32..48).contains(&vector));
    // SAFETY: vector 已限界，初始化阶段独占 IDT；0x8e 为 present、DPL0 中断门。
    // 中断门在进入时由 CPU 清 IF，保证同一个 IST3 不被另一个普通 IRQ 再次切入。
    unsafe {
        core::ptr::addr_of_mut!(IDT)
            .cast::<Gate>()
            .add(vector)
            .write(Gate::new(handler, 0x8e, 3));
    }
}

pub unsafe fn init() {
    // SAFETY: 中断关闭且单核，所有门写入完毕后才执行 lidt。
    unsafe {
        let idt = core::ptr::addr_of_mut!(IDT).cast::<Gate>();
        for vector in 0..256 {
            let entry = if vector < 32 {
                core::ptr::addr_of!(exception_entries)
                    .cast::<u64>()
                    .add(vector)
                    .read()
            } else {
                exception_255 as *const () as u64
            };
            // 3/4/5 保留原教学版 DPL=3 的系统陷阱门，NMI 为中断门。
            let attributes = match vector {
                3..=5 => 0xef,
                2 => 0x8e,
                _ => 0x8f,
            };
            idt.add(vector).write(Gate::new(
                entry,
                attributes,
                if vector == 8 { 2 } else { 1 },
            ));
        }
        let pointer = TablePointer {
            limit: (core::mem::size_of::<[Gate; 256]>() - 1) as u16,
            base: idt as u64,
        };
        asm!("lidt [{}]", in(reg) &pointer, options(readonly, nostack, preserves_flags));
    }
}

#[unsafe(no_mangle)]
extern "C" fn exception_dispatch(frame: &InterruptFrame) -> ! {
    const NAMES: [&str; 32] = [
        "divide error",
        "debug",
        "NMI",
        "breakpoint",
        "overflow",
        "bounds",
        "undefined opcode",
        "device not available",
        "double fault",
        "coprocessor segment overrun",
        "invalid TSS",
        "segment not present",
        "stack segment fault",
        "general protection",
        "page fault",
        "reserved",
        "x87 FPU",
        "alignment check",
        "machine check",
        "SIMD",
        "virtualization",
        "control protection",
        "reserved",
        "reserved",
        "reserved",
        "reserved",
        "reserved",
        "reserved",
        "hypervisor injection",
        "VMM communication",
        "security",
        "reserved",
    ];
    let name = NAMES
        .get(frame.vector as usize)
        .copied()
        .unwrap_or("unknown interrupt");
    kprint!(
        console::RED,
        "EXCEPTION {} ({name}) ERROR={:#018x} RIP={:#018x} RSP={:#018x}\n",
        frame.vector,
        frame.error,
        frame.rip,
        frame.rsp
    );
    if (10..=13).contains(&frame.vector) {
        let table = if frame.error & 2 != 0 {
            "IDT"
        } else if frame.error & 4 != 0 {
            "LDT"
        } else {
            "GDT"
        };
        kprint!(
            console::RED,
            "Selector index={} table={table} external={}\n",
            frame.error >> 3,
            frame.error & 1 != 0
        );
    }
    if frame.vector == 14 {
        kprint!(
            console::RED,
            "CR2={:#018x} present={} write={} user={} reserved={} instruction_fetch={}\n",
            arch::cr2(),
            frame.error & 1 != 0,
            frame.error & 2 != 0,
            frame.error & 4 != 0,
            frame.error & 8 != 0,
            frame.error & 16 != 0
        );
    }
    if frame.cs & 3 == 3 {
        kprint!(
            console::RED,
            "User exception context: CS={:#06x} SS={:#06x} RFLAGS={:#018x}\n",
            frame.cs,
            frame.ss,
            frame.rflags
        );
        #[cfg(any(
            feature = "fault-user-kernel",
            feature = "fault-user-write",
            feature = "fault-user-exec",
            feature = "fault-user-privileged"
        ))]
        // SAFETY: 致命异常入口已关 IF，单核；只读本节演示的状态和 CPU 保存现场。
        unsafe {
            crate::usermode::verify_expected_fault(frame)
        };
    }
    arch::halt()
}
