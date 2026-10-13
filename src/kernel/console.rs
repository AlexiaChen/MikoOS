//! VBE 线性帧缓冲上的 8×16 字符输出，无堆分配。
//! core::fmt 替代 C 的可变参数和无长度上限 vsprintf 缓冲区。
use core::{
    cell::UnsafeCell,
    fmt::{self, Write},
    ptr::{read_unaligned, write_volatile},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{arch, font::FONT, memory::PAGE_OFFSET};

pub const RED: u32 = 0x00ff0000;
pub const ORANGE: u32 = 0x00ff8000;
pub const YELLOW: u32 = 0x00ffff00;
pub const GREEN: u32 = 0x0000ff00;
pub const BLUE: u32 = 0x000000ff;
pub const INDIGO: u32 = 0x0000ffff;
pub const PURPLE: u32 = 0x008000ff;

struct Console {
    buffer: *mut u32,
    width: usize,
    height: usize,
    stride: usize,
    x: usize,
    y: usize,
}

struct SharedConsole(UnsafeCell<Console>);
// SAFETY: 所有访问都由 BUSY 的 acquire/release 保护；异常重入不等待锁。
unsafe impl Sync for SharedConsole {}
static BUSY: AtomicBool = AtomicBool::new(false);
static CONSOLE: SharedConsole = SharedConsole(UnsafeCell::new(Console {
    buffer: core::ptr::null_mut(),
    width: 0,
    height: 0,
    stride: 0,
    x: 0,
    y: 0,
}));

pub fn init() {
    // SAFETY: Loader 的 VBE ModeInfoBlock 位于直映的物理 0x8200，初始化时单核无中断。
    unsafe {
        let info = (PAGE_OFFSET + 0x8200) as *const u8;
        let word = |offset| read_unaligned(info.add(offset).cast::<u16>()) as usize;
        let width = word(18);
        let height = word(20);
        let pitch = word(16);
        let physical = read_unaligned(info.add(40).cast::<u32>()) as usize;
        // 当前字体绘制支持标准 BGRX8888；head.S 为显存保留 8 个 2 MiB PDE。
        assert!(word(0) & 0x81 == 0x81, "VBE mode lacks linear framebuffer");
        assert!(info.add(25).read() == 32 && info.add(27).read() == 6);
        assert!(info.add(31).read() == 8 && info.add(32).read() == 16);
        assert!(info.add(33).read() == 8 && info.add(34).read() == 8);
        assert!(info.add(35).read() == 8 && info.add(36).read() == 0);
        assert!(width >= 8 && height >= 16 && pitch >= width * 4 && pitch % 4 == 0);
        let offset = physical & ((1 << 21) - 1);
        assert!(physical != 0 && offset + pitch * height <= 16 * 1024 * 1024);
        *CONSOLE.0.get() = Console {
            buffer: (PAGE_OFFSET + 0xa00000 + offset) as *mut u32,
            width,
            height,
            stride: pitch / 4,
            x: 0,
            y: 0,
        };
    }
    let mut debug = DebugWriter;
    let _ = writeln!(debug, "VBE console ready");
}

impl Console {
    fn newline(&mut self) {
        self.x = 0;
        // 教材当前阶段采用回到顶部覆盖；滚屏功能留给后续章节。
        self.y = (self.y + 1) % (self.height / 16);
    }

    fn glyph(&mut self, byte: u8, color: u32) {
        for (row, bits) in FONT[byte as usize].iter().enumerate() {
            for column in 0..8 {
                let pixel = if bits & (0x80 >> column) != 0 {
                    color
                } else {
                    0
                };
                let index = (self.y * 16 + row) * self.stride + self.x * 8 + column;
                // SAFETY: x/y 以字符格为单位，换行时限界；VBE pitch 和映射长度已验证。
                // MMIO 必须 volatile，避免编译器省略硬件可见写入。
                unsafe { write_volatile(self.buffer.add(index), pixel) };
            }
        }
    }

    fn byte(&mut self, byte: u8, color: u32) {
        if self.buffer.is_null() {
            return;
        }
        match byte {
            b'\n' => self.newline(),
            b'\r' => self.x = 0,
            b'\t' => {
                let count = 8 - self.x % 8;
                for _ in 0..count {
                    self.byte(b' ', color);
                }
            }
            8 => {
                if self.x > 0 {
                    self.x -= 1;
                } else if self.y > 0 {
                    self.y -= 1;
                    self.x = self.width / 8 - 1;
                }
                self.glyph(b' ', color);
            }
            _ => {
                self.glyph(byte, color);
                self.x += 1;
                if self.x >= self.width / 8 {
                    self.newline();
                }
            }
        }
    }
}

struct DebugWriter;
impl Write for DebugWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            arch::debug_byte(byte);
        }
        Ok(())
    }
}

struct ScreenWriter<'a> {
    console: &'a mut Console,
    color: u32,
}
impl Write for ScreenWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.console.byte(byte, self.color);
        }
        Ok(())
    }
}

pub fn print(color: u32, args: fmt::Arguments<'_>) {
    // 始终先写调试端口。若绘制触发异常，仍有可读日志且不会递归等待同一把锁。
    let _ = DebugWriter.write_fmt(args);
    if BUSY
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_ok()
    {
        // SAFETY: 只有成功持有 BUSY 的执行流才能构造可变引用；重入者跳过绘制。
        let console = unsafe { &mut *CONSOLE.0.get() };
        let _ = ScreenWriter { console, color }.write_fmt(args);
        BUSY.store(false, Ordering::Release);
    }
}

#[macro_export]
macro_rules! kprint {
    ($color:expr, $($arg:tt)*) => {
        $crate::console::print($color, core::format_args!($($arg)*))
    };
}
