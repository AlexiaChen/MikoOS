# 架构与源码导航

```text
BIOS
└── src/boot/boot.asm              FAT12 → LOADER.BIN
    └── src/boot/loader.asm        KERNEL.BIN → 1 MiB，E820/VBE，进入 long mode
        └── src/kernel/head.S     CR3/GDT，高半区，BSS，内核栈，CALL
            └── main.rs::Start_Kernel
                ├── console.rs::init / kprint!
                ├── gate.rs::init → trap.rs::init
                ├── processor.rs::verify_boot_mode → 只读核对模式/代码段/分页根/高半区
                ├── memory.rs::init → bootinfo.rs 交接/布局校验 → memory/allocator.rs 分配/释放演示
                ├── interrupt.rs::init → demonstrate_timer
                ├── task.rs::demonstrate → init 让出/恢复/退出 → 启动任务回收
                ├── usermode.rs::demonstrate → SYSEXIT64 → Rust ring3 → SYSENTER 服务/返回 → 完成通知 → 清理
                └── keyboard.rs::init → ready → run → STI/HLT 等待 IRQ1
```

- [启动与交接协议](boot.md)：第 7 章实模式读盘/保护模式复制、A20 检查、物理布局与汇编边界。
- [E820 交接与引导布局](bootinfo.md)：完整版本头、固件返回规范化、区间与布局校验。
- [处理器体系结构](processor.md)：实模式/保护模式/IA-32e、地址变换和启动状态检查。
- [图形控制台](console.md)：VBE、字体、边界与格式化。
- [异常](exceptions.md)：GDT/IDT/TSS、IST、栈 ABI、诊断。
- [物理内存](memory.md)：E820、元数据、位图/Page/Zone、连续页分配/释放、分页边界。
- [外部中断](interrupts.md)：8259A/PIT、IRQ 门/独立栈、EOI、IRETQ 和验证窗口。
- [键盘](keyboard.md)：8042 初始化、IRQ1、扫描码队列、交互与输入验证。
- [任务](tasks.md)：首个内核线程、独立栈、C ABI 主动切换、TSS RSP0 和退出回收。
- [用户态](usermode.md)：SYSEXIT64、用户页权限、TSS 返回栈与有限完成通知。
- [系统调用](syscall.md)：SYSENTER/SYSEXIT64、128 槽分发、文字输出和用户参数检查。

Rust 无 `std`、无堆分配器；`arch.rs` 是特权指令和裸机内存函数的薄封装。
构建系统与模拟器不属于内核逻辑，参见 [构建](../development/build.md)。
