//! 第 4 章 4-10：捕获 PS/2 键盘的扫描码，不把它误当 ASCII 字符。
//!
//! 按键先由键盘编码，经传统 8042 控制器放到输出缓冲，IRQ1 再通知 CPU 来取。
//! 扫描码表达的是“哪个物理键发生什么变化”：按下/松开都有数据，扩展键还有
//! E0/E1 等前缀。把一个字节直接当字符，会丢失这些语义；本节先原样显示字节，
//! 组合键、字符布局和完整扫描码解析留给后续键盘驱动章节。
use crate::{arch, console, interrupt, kprint};
use core::ptr::addr_of_mut;

const DATA: u16 = 0x60;
const STATUS_COMMAND: u16 = 0x64;
const OUTPUT_FULL: u8 = 1;
const INPUT_FULL: u8 = 1 << 1;
const AUXILIARY: u8 = 1 << 5;
const BAD_DATA: u8 = (1 << 6) | (1 << 7);
// 这是有限的轮询预算，不是毫秒计时。设备不响应应停止初始化，不能无限卡死。
const POLL_BUDGET: usize = 100_000;
const QUEUE_CAPACITY: usize = 64;

/// IRQ 只读取设备并入队，主流程负责格式化输出。有限队列吸收瞬时输入突发，
/// 但不是无限存储：满时丢弃新字节并累计计数，保留已有字节的顺序，绝不覆盖未消费数据。
struct ByteQueue {
    bytes: [u8; QUEUE_CAPACITY],
    head: usize,
    length: usize,
    dropped: usize,
    errors: usize,
}
impl ByteQueue {
    const fn new() -> Self {
        Self {
            bytes: [0; QUEUE_CAPACITY],
            head: 0,
            length: 0,
            dropped: 0,
            errors: 0,
        }
    }

    fn push(&mut self, byte: u8) {
        if self.length == QUEUE_CAPACITY {
            self.dropped += 1;
            return;
        }
        self.bytes[(self.head + self.length) % QUEUE_CAPACITY] = byte;
        self.length += 1;
    }

    fn pop(&mut self) -> Option<u8> {
        if self.length == 0 {
            return None;
        }
        let byte = self.bytes[self.head];
        self.head = (self.head + 1) % QUEUE_CAPACITY;
        self.length -= 1;
        Some(byte)
    }
}
static mut QUEUE: ByteQueue = ByteQueue::new();

unsafe fn wait_for_input_empty() {
    // SAFETY: 调用者独占 8042。状态 bit1=0 才能写命令或数据，避免覆盖旧命令。
    unsafe {
        for _ in 0..POLL_BUDGET {
            if arch::in8(STATUS_COMMAND) & INPUT_FULL == 0 {
                return;
            }
            core::hint::spin_loop();
        }
    }
    panic!("8042 input buffer timeout");
}

unsafe fn command(value: u8) {
    // SAFETY: 初始化流程独占控制器；写之前等待旧输入被设备取走。
    unsafe {
        wait_for_input_empty();
        arch::out8(STATUS_COMMAND, value);
    }
}

unsafe fn write_data(value: u8) {
    // SAFETY: 调用者决定该字节是控制器参数还是发给键盘的命令，并保持顺序。
    unsafe {
        wait_for_input_empty();
        arch::out8(DATA, value);
    }
}

unsafe fn read_reply() -> u8 {
    // SAFETY: 仅在初始化的命令/应答阶段使用；PIC IRQ1 屏蔽，没有中断争抢数据。
    unsafe {
        for _ in 0..POLL_BUDGET {
            let status = arch::in8(STATUS_COMMAND);
            if status & OUTPUT_FULL != 0 {
                let byte = arch::in8(DATA);
                assert_eq!(status & (AUXILIARY | BAD_DATA), 0, "invalid 8042 reply");
                return byte;
            }
            core::hint::spin_loop();
        }
    }
    panic!("8042 output buffer timeout");
}

unsafe fn keyboard_command(value: u8) {
    // SAFETY: 当前独占键盘命令流，每个字节都等 ACK 后才发下一个；有限重试 RESEND。
    unsafe {
        for _ in 0..3 {
            write_data(value);
            match read_reply() {
                0xfa => return,
                0xfe => continue,
                reply => panic!("unexpected PS/2 ACK: {reply:#04x}"),
            }
        }
    }
    panic!("PS/2 keyboard keeps requesting resend");
}

/// # Safety
/// 单核 IF=0，PIC/IDT 已初始化且全屏蔽；独占传统 PC 的 8042 及第一 PS/2 端口。
pub unsafe fn init() {
    assert!(!arch::interrupts_enabled());
    // SAFETY: 全部命令在关中断、IRQ1 屏蔽时完成；普通接收路径尚未运行。
    unsafe {
        command(0xad); // 禁用第一端口，先阻止新数据进入初始化的应答流。
        command(0xa7); // 本节没有鼠标驱动，第二端口保持禁用。
        // BIOS 可能留下扫描码或应答。先消费旧输出，再开始新的命令事务，
        // 否则把旧按键误当配置字节/ACK，会让后续控制器状态全部错位。
        for _ in 0..QUEUE_CAPACITY {
            if arch::in8(STATUS_COMMAND) & OUTPUT_FULL == 0 {
                break;
            }
            let _ = arch::in8(DATA);
        }
        assert_eq!(
            arch::in8(STATUS_COMMAND) & OUTPUT_FULL,
            0,
            "8042 stale output did not drain"
        );
        command(0x20); // 读控制器配置字节；与 PIC 命令端口 0x20 是不同概念。
        let configuration = read_reply();
        // 配置 bit6 开启 set2→set1 翻译；bit5 禁用第二端口时钟；bit4 清零开放
        // 第一端口时钟。bit0/1 是控制器发 IRQ 的开关，与 PIC IMR/CPU IF 又是三层。
        // 暂时关闭两路 IRQ，先收完键盘命令 ACK，避免把 ACK 混入扫描码队列。
        let configuration = (configuration | (1 << 6) | (1 << 5)) & !(1 | 2 | (1 << 4));
        command(0x60);
        write_data(configuration);
        command(0xae);
        // 明确选择键盘线上 set2，再由控制器翻译成 set1；不依赖 BIOS 留下的模式。
        // F5 暂停扫描后再配置，避免用户按键字节与命令应答混杂；F4 最后恢复扫描。
        keyboard_command(0xf5);
        keyboard_command(0xf0);
        keyboard_command(0x02);
        keyboard_command(0xf4);
        command(0x60);
        write_data(configuration | 1);
        #[cfg(feature = "keyboard-check")]
        verify_queue();
        interrupt::enable_keyboard();
    }
    kprint!(
        console::GREEN,
        "PS/2 keyboard ready: IRQ1, translated set 1 scan bytes\n"
    );
}

/// # Safety
/// 只在 IRQ1 入口调用，单核 IF=0、不可嵌套；主流程不持有队列引用。
pub unsafe fn on_interrupt() {
    // SAFETY: IRQ 独占 8042 数据端口与 QUEUE；借用在函数返回前结束，不跨 STI。
    unsafe {
        let queue = &mut *addr_of_mut!(QUEUE);
        let status = arch::in8(STATUS_COMMAND);
        if status & OUTPUT_FULL == 0 {
            queue.errors += 1;
            return;
        }
        let byte = arch::in8(DATA);
        if status & (AUXILIARY | BAD_DATA) != 0 {
            queue.errors += 1;
            return;
        }
        queue.push(byte);
    }
}

/// 这是可以被设备唤醒的等待，不是 CLI/HLT 的最终停机。保留符号供普通 smoke
/// 在“已准备接收键盘输入”的边界退出；实际 IRQ1 交付由 keyboard-check 单独验证。
///
/// # Safety
/// IF=0，键盘入口就绪，没有活跃的队列引用；仅本模块接收循环调用。
#[unsafe(export_name = "kernel_keyboard_wait")]
#[inline(never)]
unsafe fn wait() {
    // SAFETY: 主循环已经消费完队列并结束借用；STI/HLT 原子等待避免漏掉唤醒。
    unsafe { arch::wait_for_interrupt() }
}

/// # Safety
/// init 完成，单核 IF=0，无其他队列使用者；只开放键盘 IRQ1，主循环消费字节。
pub unsafe fn run() -> ! {
    #[cfg(feature = "keyboard-check")]
    let mut checked = 0;
    // SAFETY: 队列只在 IF=0 时访问，且引用不跨 wait()；IRQ 不可能与当前访问重叠。
    unsafe {
        loop {
            while let Some(byte) = (&mut *addr_of_mut!(QUEUE)).pop() {
                kprint!(console::YELLOW, "Keyboard scan byte: {byte:#04x}\n");
                #[cfg(feature = "keyboard-check")]
                {
                    // 由宿主经 VNC 发 a 按下/松开、b 按下/松开；这四个字节必须
                    // 经过模拟键盘、8042、PIC、IRQ1 和队列，不能由内核自己塞入。
                    const EXPECTED: [u8; 4] = [0x1e, 0x9e, 0x30, 0xb0];
                    assert_eq!(byte, EXPECTED[checked]);
                    checked += 1;
                    if checked == EXPECTED.len() {
                        interrupt::stop_keyboard_check();
                        let queue = &*addr_of_mut!(QUEUE);
                        assert_eq!((queue.length, queue.dropped, queue.errors), (0, 0, 0));
                        kprint!(
                            console::GREEN,
                            "Keyboard IRQ1 checks OK: a make/break, b make/break; no drops/errors\n"
                        );
                        arch::halt();
                    }
                }
            }
            // 只有关中断访问队列这一前提，才能安全使用一个普通环形缓冲而不加锁。
            // 本节单核、IRQ 非嵌套，NMI 不操作队列；SMP 或中断中再开 IF 时须重做同步。
            {
                let queue = &mut *addr_of_mut!(QUEUE);
                if queue.dropped != 0 || queue.errors != 0 {
                    kprint!(
                        console::RED,
                        "Keyboard input lost: dropped={} errors={}\n",
                        queue.dropped,
                        queue.errors
                    );
                    queue.dropped = 0;
                    queue.errors = 0;
                }
            }
            wait();
        }
    }
}

#[cfg(feature = "keyboard-check")]
fn verify_queue() {
    let mut queue = ByteQueue::new();
    for byte in 0..QUEUE_CAPACITY {
        queue.push(byte as u8);
    }
    queue.push(0xff);
    assert_eq!(queue.dropped, 1);
    for byte in 0..QUEUE_CAPACITY / 2 {
        assert_eq!(queue.pop(), Some(byte as u8));
    }
    for byte in QUEUE_CAPACITY..QUEUE_CAPACITY + QUEUE_CAPACITY / 2 {
        queue.push(byte as u8);
    }
    for byte in QUEUE_CAPACITY / 2..QUEUE_CAPACITY + QUEUE_CAPACITY / 2 {
        assert_eq!(queue.pop(), Some(byte as u8));
    }
    assert_eq!(queue.pop(), None);
    kprint!(
        console::GREEN,
        "Keyboard queue checks OK: wraparound, FIFO, overflow\n"
    );
}
