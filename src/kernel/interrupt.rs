//! 第 4 章 4-9：设备 → PIC → IDT → 保存现场 → Rust → EOI → IRETQ。
//!
//! 异常来自当前指令的执行，中断则可在两条指令之间异步到来。异常诊断可以停机，
//! 设备 IRQ 必须能回到被打断的位置，不能依赖普通函数的 RET 或调用者保存寄存器。
//! 先用 PIT IRQ0 完成两次交付演示，再开放键盘 IRQ1；没有时钟服务或任务调度。
mod pic;

use crate::{arch, console, keyboard, kprint, trap, trap::InterruptFrame};
use core::sync::atomic::{AtomicUsize, Ordering};

static TIMER_IRQS: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "irq-check")]
static SPURIOUS_MASTER: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "irq-check")]
static SPURIOUS_SLAVE: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" {
    static irq_entries: [u64; pic::IRQ_COUNT];
    #[cfg(feature = "irq-check")]
    fn irq_verify_context() -> u64;
}

/// # Safety
/// TSS/IDT 已加载，单核 IF=0；之后本模块独占 PIC 和 PIT channel 0。
pub unsafe fn init() {
    assert!(!arch::interrupts_enabled());
    // SAFETY: 按初始化契约串行安装入口；所有门就绪后才重设 PIC，保持全屏蔽。
    unsafe {
        for irq in 0..pic::IRQ_COUNT {
            let entry = core::ptr::addr_of!(irq_entries)
                .cast::<u64>()
                .add(irq)
                .read();
            trap::install_irq(pic::VECTOR_BASE + irq, entry);
        }
        pic::init();
        assert_eq!(pic::in_service(), 0);
    }
    kprint!(
        console::GREEN,
        "8259A ready: vectors=0x20..0x2f, all IRQs masked\n"
    );
}

/// # Safety
/// init 已完成，IF=0，没有持有中断处理程序会访问的可变引用/资源；仅启动时调用一次。
pub unsafe fn demonstrate_timer() {
    assert!(!arch::interrupts_enabled());
    assert_eq!(TIMER_IRQS.load(Ordering::Relaxed), 0);
    // SAFETY: 独占 PIT channel 0，其他 IRQ 始终屏蔽；此函数返回时保持 IF=0。
    unsafe {
        // PIT 的输入时钟约为 1,193,182 Hz。除数 11,932 得到约 100 Hz 的周期脉冲。
        // 设置频率仅为得到稳定可复现的外部请求，不引入 tick 调度或软件时间。
        // 0x34: channel0、先低后高字节、mode2 rate generator、二进制计数。
        const DIVISOR: u16 = 11_932;
        arch::out8(0x43, 0x34);
        arch::out8(0x40, DIVISOR as u8);
        arch::out8(0x40, (DIVISOR >> 8) as u8);
        pic::unmask_timer();
        while TIMER_IRQS.load(Ordering::Relaxed) < 2 {
            arch::wait_for_interrupt();
        }
        pic::mask_all();
        assert_eq!(pic::in_service(), 0);
    }
    assert!(!arch::interrupts_enabled());
    kprint!(
        console::GREEN,
        "PIT IRQ0 delivered twice; PIC EOI and IRETQ return OK\n"
    );
}

/// # Safety
/// 单核 IF=0，键盘初始化和 IRQ0 演示均已完成，主流程准备接收键盘字节。
pub unsafe fn enable_keyboard() {
    assert!(!arch::interrupts_enabled());
    // SAFETY: 控制器命令应答已消费，IRQ1 处理函数会先读设备数据再发 PIC EOI。
    unsafe { pic::unmask_keyboard() }
}

/// # Safety
/// IF=0 或不可嵌套 IRQ 内调用，当前执行流独占 PIC 配置。
#[cfg(feature = "keyboard-check")]
pub unsafe fn stop_keyboard_check() {
    assert!(!arch::interrupts_enabled());
    // SAFETY: 检查完毕后屏蔽所有普通 IRQ，不留下继续执行的设备处理窗口。
    unsafe { pic::mask_all() }
}

#[unsafe(no_mangle)]
extern "C" fn irq_dispatch(frame: &InterruptFrame) {
    assert!(!arch::interrupts_enabled(), "IRQ handler must not nest");
    let irq = (frame.vector as usize)
        .checked_sub(pic::VECTOR_BASE)
        .filter(|irq| *irq < pic::IRQ_COUNT)
        .expect("invalid PIC vector");
    // SAFETY: CPU 已通过中断门关 IF，所有 IRQ 用 IST3 且不嵌套；独占 PIC。
    unsafe {
        if pic::is_spurious(irq) {
            #[cfg(feature = "irq-check")]
            if irq == 7 {
                SPURIOUS_MASTER.fetch_add(1, Ordering::Relaxed);
            } else {
                SPURIOUS_SLAVE.fetch_add(1, Ordering::Relaxed);
            }
            return;
        }
        // 软件 INT 同样能查 IDT，却没有 PIC 的应答/ISR 周期，不能用它证明外部
        // IRQ 已工作。显式核对 ISR 再计数，确保本演示收到的是控制器交付的请求。
        assert!(irq <= 1, "device IRQ has no driver yet");
        assert_ne!(pic::in_service() & (1 << irq), 0, "IRQ must come from PIC");
        // 只有单核处理函数写计数，主流程关中断时读取；这里只传递计数本身，
        // 不靠它发布其他内存内容，所以 Relaxed 足够。原子类型也避免 static mut 数据竞争。
        if irq == 0 {
            let count = TIMER_IRQS.fetch_add(1, Ordering::Relaxed) + 1;
            assert!(count <= 2);
            if count == 2 {
                pic::mask_all();
            }
        } else {
            // EOI 只结束 PIC 的服务，不能替键盘清空输出缓冲。必须先读走 0x60
            // 中的数据，让设备撤销请求，之后的新字节才能形成新的 IRQ 边沿。
            keyboard::on_interrupt();
        }
        pic::end_of_interrupt(irq);
        assert_eq!(pic::in_service() & (1 << irq), 0, "EOI must clear ISR");
    }
    // 不在高频 IRQ 路径输出文字；主流程恢复后再报告，以免打断同一条日志或持锁重入。
}

/// # Safety
/// init 已完成，IF=0，PIC 所有线屏蔽且 ISR 为空；本节验证专用。
#[cfg(feature = "irq-check")]
pub unsafe fn verify_context() {
    assert!(!arch::interrupts_enabled());
    // SAFETY: 软件 INT 仅走已安装的伪 IRQ7/15 处理路径；无设备服务位，无实际设备访问。
    unsafe {
        assert_eq!(pic::in_service(), 0);
        assert_eq!(irq_verify_context(), 0, "IRQ clobbered GPRs/RFLAGS/RSP");
        core::arch::asm!("int 0x2f");
        assert_eq!(SPURIOUS_MASTER.load(Ordering::Relaxed), 1);
        assert_eq!(SPURIOUS_SLAVE.load(Ordering::Relaxed), 1);
        assert_eq!(TIMER_IRQS.load(Ordering::Relaxed), 0);
        assert_eq!(pic::in_service(), 0);
    }
    kprint!(
        console::GREEN,
        "IRQ context checks OK: GPRs, RFLAGS, RSP; software IRQ7/15 spurious paths\n"
    );
}
