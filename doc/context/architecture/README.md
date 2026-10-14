# 架构与源码导航

```text
BIOS
└── src/boot/boot.asm              FAT12 → LOADER.BIN
    └── src/boot/loader.asm        KERNEL.BIN → 1 MiB，E820/VBE，进入 long mode
        └── src/kernel/head.S     CR3/GDT，高半区，BSS，内核栈，CALL
            └── main.rs::Start_Kernel
                ├── console.rs::init / kprint!
                ├── gate.rs::init → trap.rs::init
                ├── memory.rs::init → memory/allocator.rs 分配/释放演示
                ├── interrupt.rs::init → demonstrate_timer
                ├── task.rs::demonstrate → init 让出/恢复/退出 → 启动任务回收
                └── keyboard.rs::init → ready → run → STI/HLT 等待 IRQ1
```

- [启动与交接协议](boot.md)：物理地址、链接布局、汇编保留原因。
- [图形控制台](console.md)：VBE、字体、边界与格式化。
- [异常](exceptions.md)：GDT/IDT/TSS、IST、栈 ABI、诊断。
- [物理内存](memory.md)：E820、元数据、位图/Page/Zone、连续页分配/释放、分页边界。
- [外部中断](interrupts.md)：8259A/PIT、IRQ 门/独立栈、EOI、IRETQ 和验证窗口。
- [键盘](keyboard.md)：8042 初始化、IRQ1、扫描码队列、交互与输入验证。
- [任务](tasks.md)：首个内核线程、独立栈、C ABI 主动切换、TSS RSP0 和退出回收。

Rust 无 `std`、无堆分配器；`arch.rs` 是特权指令和裸机内存函数的薄封装。
构建系统与模拟器不属于内核逻辑，参见 [构建](../development/build.md)。
