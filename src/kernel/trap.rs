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
pub struct ExceptionFrame {
    registers: [u64; 15], // r15..r8, rsi,rdi,rbp,rdx,rcx,rbx,rax
    vector: u64,
    error: u64,
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}
const _: () = assert!(core::mem::offset_of!(ExceptionFrame, rip) == 136);

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
extern "C" fn exception_dispatch(frame: &ExceptionFrame) -> ! {
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
    arch::halt()
}
