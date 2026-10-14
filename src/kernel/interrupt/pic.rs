//! 两片级联的 8259A：把设备的 IRQ 线转换成 CPU 能用来查 IDT 的向量号。
//!
//! 设备、PIC、CPU 各管一层：设备产生请求；PIC 用 IRR 记录待处理请求、用
//! IMR 屏蔽暂不接收的线、用 ISR 记录正在服务的线；CPU 的 RFLAGS.IF 决定
//! 是否接受可屏蔽中断。CLI 不是“设备不再发 IRQ”，屏蔽 PIC 也不是清空设备。
use crate::arch;

pub const VECTOR_BASE: usize = 0x20;
pub const IRQ_COUNT: usize = 16;
const MASTER_COMMAND: u16 = 0x20;
const MASTER_DATA: u16 = 0x21;
const SLAVE_COMMAND: u16 = 0xa0;
const SLAVE_DATA: u16 = 0xa1;

/// # Safety
/// 初始化阶段独占 PIC，IF=0；端口属于传统 PC 的两片 8259A。
pub unsafe fn init() {
    // SAFETY: 调用者独占设备；ICW 顺序、端口、级联关系按 PC/AT 接线设置。
    unsafe {
        mask_all();
        // ICW1: bit4 开始初始化；bit0 表示还有 ICW4；未置位的 bit3 选择边沿触发，
        // bit1=0 表示级联。每次写后用传统 POST 端口等待，兼容较慢的端口总线。
        write_initialization(MASTER_COMMAND, 0x11);
        write_initialization(SLAVE_COMMAND, 0x11);
        // ICW2: PIC 提供向量的高 5 位，低 3 位来自 IRQ 输入号，基址须 8 对齐。
        // BIOS 的历史向量分配会与 CPU 的 0..31 异常重叠，内核必须重新定位。
        write_initialization(MASTER_DATA, VECTOR_BASE as u8);
        write_initialization(SLAVE_DATA, (VECTOR_BASE + 8) as u8);
        // ICW3 两片含义不同：主片是“哪些输入连着从片”的位图，从片是级联线编号。
        // 主 IRQ2 连接从片 INT，所以外部 IRQ8..15 要经过两级控制器。
        write_initialization(MASTER_DATA, 1 << 2);
        write_initialization(SLAVE_DATA, 2);
        // ICW4: 8086 模式，普通 EOI，禁用 Auto-EOI，以便观察完整的服务生命周期。
        write_initialization(MASTER_DATA, 0x01);
        write_initialization(SLAVE_DATA, 0x01);
        // 未有驱动的线保持屏蔽。不能照搬原版全部开放，否则键盘、RTC 等请求
        // 可能无人清除设备状态，造成中断风暴或让初始化过程重入。
        mask_all();
        assert_eq!(arch::in8(MASTER_DATA), 0xff);
        assert_eq!(arch::in8(SLAVE_DATA), 0xff);
    }
}

unsafe fn write_initialization(port: u16, value: u8) {
    // SAFETY: 调用者遵守 PIC ICW 序列；0x80 在本节传统 PC 平台用于 I/O 延迟。
    unsafe {
        arch::out8(port, value);
        arch::out8(0x80, 0);
    }
}

/// # Safety
/// IF=0 或处于不可嵌套的 IRQ 处理函数中，当前执行流独占 PIC 配置。
pub unsafe fn mask_all() {
    // SAFETY: OCW1 的 1 位表示屏蔽，对两片数据端口各写 0xff。
    unsafe {
        arch::out8(MASTER_DATA, 0xff);
        arch::out8(SLAVE_DATA, 0xff);
    }
}

/// # Safety
/// PIC 初始化完成且 IF=0；IRQ0 的入口及处理函数已就绪。
pub unsafe fn unmask_timer() {
    // SAFETY: 只开放主片 bit0；从片及其级联线 IRQ2 均保持屏蔽。
    unsafe { arch::out8(MASTER_DATA, 0xfe) }
}

/// # Safety
/// 独占 PIC 端口访问；读取命令端口前先用 OCW3 明确选择 ISR。
pub unsafe fn in_service() -> u16 {
    // SAFETY: OCW3 0x0b 的读寄存器选择是 ISR，不是默认的 IRR；只读状态不清除位。
    unsafe {
        arch::out8(MASTER_COMMAND, 0x0b);
        arch::out8(SLAVE_COMMAND, 0x0b);
        u16::from(arch::in8(MASTER_COMMAND)) | (u16::from(arch::in8(SLAVE_COMMAND)) << 8)
    }
}

/// PIC 的 INT 已被 CPU 接受，但请求在应答前消失时，会交出该片最后一个向量。
/// 于是 IRQ7/IRQ15 可能并没有真正进入 ISR，不能把它们一律当成设备事件。
///
/// # Safety
/// irq 在 0..16，IF=0，独占 PIC；此函数包含伪 IRQ15 的主片 EOI。
pub unsafe fn is_spurious(irq: usize) -> bool {
    assert!(irq < IRQ_COUNT);
    // SAFETY: ISR 按硬件服务状态判定，且只对特殊的 IRQ7/15 使用此协议。
    unsafe {
        if (irq == 7 || irq == 15) && in_service() & (1 << irq) == 0 {
            if irq == 15 {
                // 从片没有真正服务请求，但主片可能已应答级联 IRQ2；只结束主片
                // IRQ2，不能向从片发送 EOI。指定线号的 EOI 不误清其他在服务位。
                arch::out8(MASTER_COMMAND, 0x60 | 2);
            }
            return true;
        }
    }
    false
}

/// # Safety
/// irq 是已进入 ISR 的真实 IRQ；设备处理结束，不是软件 INT 或伪中断。
pub unsafe fn end_of_interrupt(irq: usize) {
    assert!(irq < IRQ_COUNT);
    // SAFETY: OCW2 的 0x60 是 specific EOI，低三位指定结束服务的输入线。
    unsafe {
        if irq >= 8 {
            // 级联请求占用了两片 ISR，先结束从片实际输入，再结束主片 IRQ2。
            // 只给主片 EOI 会让从片继续认为旧中断在服务，之后同级/低优先级可能被阻塞。
            arch::out8(SLAVE_COMMAND, 0x60 | (irq as u8 - 8));
            arch::out8(MASTER_COMMAND, 0x60 | 2);
        } else {
            arch::out8(MASTER_COMMAND, 0x60 | irq as u8);
        }
    }
}
