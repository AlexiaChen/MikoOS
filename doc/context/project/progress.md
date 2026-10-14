# 当前进度与原版章节映射

基线为迁移前提交 `a76a0b8`。范围由原仓库 `src` 确认，不能把原版后续章节当成本项目已完成。
当前原版映射主要是 `code/c03`、`code/c04/4-4`（屏幕）、`4-5`（异常）、`4-6`～`4-8`（初级内存）、`4-9`（外部中断）、`4-10`（键盘中断）；
本项目有自己的重构和命名，不能逐文件假定完全相同。

| 原版章节/部分 | 当前实现 | 入口 |
| --- | --- | --- |
| 第 3 章 Boot | BIOS 引导扇区、FAT12 加载 Loader，NASM 保留 | `src/boot/boot.asm` |
| 第 3 章 Loader | 加载 Kernel、E820、VBE、保护模式/长模式，NASM 保留 | `src/boot/loader.asm` |
| 第 4 章 内核执行头 | 页表、GDT、高半区、BSS 清零、独立内核栈，汇编边界 | `src/kernel/head.S` |
| 第 4 章 内核主程序 | Rust 初始化调用链、panic、停机 | `src/kernel/main.rs` |
| 第 4 章 屏幕显示 | Rust 帧缓冲文字、原点阵字体、core::fmt | `src/kernel/console.rs` |
| 第 4 章 系统异常 | Rust IDT/TSS 与诊断，汇编规范化异常现场 | `gate.rs`、`trap.rs`、`entry.S` |
| 第 4 章 初级内存管理（4-6～4-8） | Rust E820、2 MiB Page/Zone、位图、预留、连续页分配/释放、移除低地址别名 | `src/kernel/memory.rs`、`memory/allocator.rs` |
| 第 4 章 外部中断（4-9） | Rust 8259A、IRQ0 时钟演示、现场恢复与 IRETQ、伪 IRQ7/15 分支 | `interrupt.rs`、`interrupt/pic.rs`、`irq.S` |
| 第 4 章 键盘中断（4-10） | Rust 8042/PS/2 初始化、IRQ1 取扫描码、有限队列、主循环显示字节 | `keyboard.rs`、`interrupt.rs` |

尚未实现：完整 RAM 映射、堆/SLAB、键盘字符/组合键/完整扫描码解析、
任务/调度、用户态、系统调用、文件系统驱动。FAT12 读取属于引导器，不是内核文件系统。

2026-10-14：对照原版 `code/c04/4-8/kernel/{memory.c,main.c}`，完成连续物理页分配与整段释放。
保留位图/Page/Zone 思路，改用有界逐位扫描和独占凭据；不照搬原版越界读取、移位 64 位、
页号偏一和假定 NORMAL 全部已映射的问题。教学注释侧重 CPU 分页、所有权及记账机制。
根 README 面向学习者介绍项目，不放 AI 工作指令；协作约束仍放 AGENTS.md。

2026-10-14：上一阶段提交为 `cf17d5e`。随后对照原版 `code/c04/4-9/kernel/{interrupt.c,entry.S,main.c}`
完成可返回 IRQ、8259A 重定位/屏蔽/EOI，以及真实 PIT IRQ0 的两次交付演示。
修正原版无驱动时全开放 IRQ、从片 EOI 不完整的问题；PIC 只有 16 路，不照搬 24 个入口。
异常仍 fatal；IRQ 用独立 IST3，完整保留 GPR/RFLAGS/栈。未加入调度、键盘驱动或时钟服务。

2026-10-14：4-9 已提交为 `2774174`。随后对照原版 `code/c04/4-10/kernel/interrupt.c`，
完成键盘 IRQ1 捕获；只对 IRQ1 读取 0x60，先检查 0x64 状态、消费设备字节，再给 PIC EOI。
新增显式 8042 配置/PS/2 ACK 处理和有限队列，中断不打印；普通内核持续等待并显示扫描码，
不提前实现完整字符输入。keyboard-check 经宿主 VNC 驱动模拟键盘，验证真实接收链路。

接下来按原版 `code/c04/4-11` 学习最初的任务运行；每次只推进一个可验证的小节。完整页表、SLAB、成熟调度器等
按后续相应章节继续，不在本次提前实现。验证状态见 [验证记录](../development/validation.md)。
