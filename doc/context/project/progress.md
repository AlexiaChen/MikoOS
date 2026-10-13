# 当前进度与原版章节映射

基线为迁移前提交 `a76a0b8`。范围由原仓库 `src` 确认，不能把原版后续章节当成本项目已完成。
当前原版映射主要是 `code/c03`、`code/c04/4-4`（屏幕）、`4-5`（异常）、`4-6`（初级内存）；
本项目有自己的重构和命名，不能逐文件假定完全相同。

| 原版章节/部分 | 当前实现 | 入口 |
| --- | --- | --- |
| 第 3 章 Boot | BIOS 引导扇区、FAT12 加载 Loader，NASM 保留 | `src/boot/boot.asm` |
| 第 3 章 Loader | 加载 Kernel、E820、VBE、保护模式/长模式，NASM 保留 | `src/boot/loader.asm` |
| 第 4 章 内核执行头 | 页表、GDT、高半区、BSS 清零、独立内核栈，汇编边界 | `src/kernel/head.S` |
| 第 4 章 内核主程序 | Rust 初始化调用链、panic、停机 | `src/kernel/main.rs` |
| 第 4 章 屏幕显示 | Rust 帧缓冲文字、原点阵字体、core::fmt | `src/kernel/console.rs` |
| 第 4 章 系统异常 | Rust IDT/TSS 与诊断，汇编规范化异常现场 | `gate.rs`、`trap.rs`、`entry.S` |
| 第 4 章 初级内存管理 | Rust E820、2 MiB Page/Zone、位图、预留、移除低地址别名 | `src/kernel/memory.rs` |

尚未实现：物理页批量分配 API、完整 RAM 映射、堆/SLAB、外部 IRQ/PIC、键盘、
任务/调度、用户态、系统调用、文件系统驱动。FAT12 读取属于引导器，不是内核文件系统。

下一次任务先按原版“初级内存管理单元”核对剩余内容，按用户指定的小节推进；
不要一次性跳到后续章节。验证状态见 [验证记录](../development/validation.md)。
