//! 64 位 IDT 门及 TSS。用明确的位域构造替代原 C 内联汇编宏。
use core::{arch::asm, ptr::write_unaligned};

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Gate {
    offset_low: u16,
    selector: u16,
    ist: u8,
    attributes: u8,
    offset_middle: u16,
    offset_high: u32,
    reserved: u32,
}

impl Gate {
    pub const EMPTY: Self = Self {
        offset_low: 0,
        selector: 0,
        ist: 0,
        attributes: 0,
        offset_middle: 0,
        offset_high: 0,
        reserved: 0,
    };

    pub fn new(address: u64, attributes: u8, ist: u8) -> Self {
        Self {
            offset_low: address as u16,
            selector: 0x08,
            ist: ist & 7,
            attributes,
            offset_middle: (address >> 16) as u16,
            offset_high: (address >> 32) as u32,
            reserved: 0,
        }
    }
}

#[repr(C, packed)]
pub struct TablePointer {
    pub limit: u16,
    pub base: u64,
}

#[repr(C, align(16))]
struct Stack<const N: usize>([u8; N]);
static mut EXCEPTION_STACK: Stack<32768> = Stack([0; 32768]);
static mut DOUBLE_FAULT_STACK: Stack<16384> = Stack([0; 16384]);
static mut TSS: [u8; 104] = [0; 104];

unsafe extern "C" {
    static mut GDT_Table: [u64; 10];
}

pub unsafe fn init() {
    // SAFETY: 单核启动且中断关闭，静态区已清零。用 raw pointer 避免静态可变引用。
    unsafe {
        let top = core::ptr::addr_of_mut!(EXCEPTION_STACK)
            .cast::<u8>()
            .add(32768) as u64;
        let double_top = core::ptr::addr_of_mut!(DOUBLE_FAULT_STACK)
            .cast::<u8>()
            .add(16384) as u64;
        let tss = core::ptr::addr_of_mut!(TSS).cast::<u8>();
        // TSS 是硬件规定的紧凑结构：RSP0 从偏移 4 开始，IST1 从偏移 36 开始。
        for offset in [4, 12, 20, 36, 52, 60, 68, 76, 84] {
            write_unaligned(tss.add(offset).cast::<u64>(), top);
        }
        write_unaligned(tss.add(44).cast::<u64>(), double_top); // IST2 专供 #DF
        write_unaligned(tss.add(102).cast::<u16>(), 104); // 无 I/O 权限位图
        let base = tss as u64;
        let low = 103 | ((base & 0xffffff) << 16) | (0x89 << 40) | (((base >> 24) & 0xff) << 56);
        let gdt = core::ptr::addr_of_mut!(GDT_Table).cast::<u64>();
        gdt.add(8).write(low);
        gdt.add(9).write(base >> 32);
        // GDT 索引 8 的选择子是 8*8=0x40；必须先写好 TSS，再加载 TR。
        asm!("ltr ax", in("ax") 0x40u16, options(nostack, preserves_flags));
    }
}

const _: () = assert!(core::mem::size_of::<Gate>() == 16);
const _: () = assert!(core::mem::size_of::<TablePointer>() == 10);
