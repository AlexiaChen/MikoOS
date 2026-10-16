//! 第 6 章：将处理器模式与当前启动链对应起来。
//!
//! “编译了 64 位指令”“请求进入 long mode”“CPU 正在执行 64 位代码”是
//! 三个不同条件。BITS/.code64 只决定汇编编码；EFER.LME 是软件请求；
//! EFER.LMA 表示 IA-32e 已激活，而当前 CS 的 L 位决定执行 64 位子模式。
//! 本模块只读现有状态，不重新开启分页，也不增加新的 CPU 功能。
use crate::{arch, console, kprint, memory};
use core::arch::asm;

const CR0_PE: u64 = 1;
const CR0_PG: u64 = 1 << 31;
const CR4_PAE: u64 = 1 << 5;
const CR4_LA57: u64 = 1 << 12;
const EFER_LME: u64 = 1 << 8;
const EFER_LMA: u64 = 1 << 10;

/// 核对 Loader → head.S → Rust 之间已经建立的模式契约。
///
/// # Safety
/// 从 head.S 的高半区入口在 CPL0、IF=0 调用；Loader 已成功访问 EFER MSR，
/// head.S 已安装 GDT、页表和栈。此检查不能保护尚未进入 Rust 的引导阶段。
pub unsafe fn verify_boot_mode() {
    let cr0: u64;
    let cr4: u64;
    let cs: u16;
    let rip: u64;
    let rsp: u64;
    let low: u32;
    let high: u32;
    // SAFETY: 上述入口保证 ring0；读取控制寄存器/MSR 不改变模式或映射。
    // RIP 相对标签取得此刻真实执行地址，不能拿高半区链接符号冒充执行证据。
    unsafe {
        asm!("mov {cr0}, cr0", "mov {cr4}, cr4", "mov {cs:x}, cs",
            "lea {rip}, [rip + 2f]", "2:", "mov {rsp}, rsp",
            cr0 = out(reg) cr0, cr4 = out(reg) cr4, cs = out(reg) cs,
            rip = out(reg) rip, rsp = out(reg) rsp,
            options(nomem, nostack, preserves_flags));
        asm!("rdmsr", in("ecx") 0xc000_0080u32, out("eax") low,
            out("edx") high, options(nomem, nostack, preserves_flags));
    }
    let efer = u64::from(low) | (u64::from(high) << 32);
    let cr3 = arch::cr3();
    assert!(!arch::interrupts_enabled());
    assert_eq!(cr0 & (CR0_PE | CR0_PG), CR0_PE | CR0_PG);
    assert_ne!(cr4 & CR4_PAE, 0);
    assert_eq!(cr4 & CR4_LA57, 0, "bootstrap requires four-level paging");
    assert_eq!(efer & (EFER_LME | EFER_LMA), EFER_LME | EFER_LMA);
    assert_eq!(cs, 0x08, "expected kernel GDT selector and CPL0");

    let attributes: u32;
    let valid: u8;
    // LAR 让 CPU 查询当前选择子的描述符访问权，而不手工解引用 GDTR.base。
    // 返回值 bit21=L、bit22=D/B、bit15=P、bit14:13=DPL、bit12=S、bit11=代码。
    // ZF=1 才表示查询成功；L=1、D=0 才是 64 位代码段。LMA=1 而 L=0 时
    // 仍可能执行兼容模式代码，因此不能只凭 EFER 就断言当前代码是 64 位。
    // LAR 查询描述符表，不直接读取 CS 隐藏缓存；head.S 远返回重载 CS 后
    // 没有修改该代码段，故此处可用它核对已加载的段配置。
    // SAFETY: cs 是刚读取的有效 GDT 选择子；LAR 仅读取访问权并改变算术标志。
    unsafe {
        asm!("lar {attributes:e}, {selector:e}", "setz {valid}",
            attributes = out(reg) attributes, selector = in(reg) u32::from(cs),
            valid = out(reg_byte) valid, options(nostack));
    }
    assert_eq!(valid, 1, "cannot query current code descriptor");
    let code64 = (1 << 21) | (1 << 15) | (1 << 12) | (1 << 11);
    let mask = code64 | (1 << 22) | (3 << 13);
    assert_eq!(
        attributes & mask,
        code64,
        "expected present ring0 L=1 D=0 code"
    );

    // CR3 存物理页表基址，不能加高半区偏移后再写入 CPU；低 12 位不是地址。
    // 0x101000 来自 head.S 的装载基址 1 MiB 加 .org 0x1000。本节只核对根，
    // 不把它当作完整的页表遍历，也不推断所有物理 RAM 都已经映射。
    assert_eq!(cr3 & 0x000f_ffff_ffff_f000, 0x101000);
    // 四级分页使用 48 位规范线性地址：bit63:48 必须全部复制 bit47。
    // 因而中间的大段数值不是有效地址；规范地址本身也不代表一定存在映射。
    for address in [rip, rsp] {
        assert!(address <= 0x0000_7fff_ffff_ffff || address >= 0xffff_8000_0000_0000);
        assert!(
            address >= memory::PAGE_OFFSET as u64,
            "expected high-half execution"
        );
    }
    kprint!(
        console::GREEN,
        "Processor registers: CR0={cr0:#018x} CR3={cr3:#018x} CR4={cr4:#018x}\n"
    );
    kprint!(
        console::GREEN,
        "Processor code: EFER={efer:#018x} CS={cs:#04x} LAR={attributes:#010x} RIP={rip:#018x} RSP={rsp:#018x}\n"
    );
    kprint!(
        console::GREEN,
        "Processor mode OK: IA-32e active, 64-bit code, CPL0; 4-level paging, canonical RIP/RSP\n"
    );
}
