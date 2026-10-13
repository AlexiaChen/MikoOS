# BIOS/FAT12 启动与 Rust 边界

源码：`src/boot/{boot.asm,fat12.inc,loader.asm}`、`src/kernel/{head.S,kernel.lds}`、
`main.rs` 的 `global_asm!`。

| 物理地址 | 用途 |
| --- | --- |
| `0x4ff0` | Loader 写入的 E820 条目数量（u16，小端） |
| `0x5000` | 最多 32 个 E820 条目，每条 20 字节 |
| `0x7c00` | BIOS 引导扇区/早期栈区域 |
| `0x7e00` | Loader 读内核时的单扇区临时区 |
| `0x8000` | VBE ControllerInfoBlock |
| `0x8200` | 最终选定 `0x180` 模式的 VBE ModeInfoBlock |
| `0x10000` | Loader 装载地址 |
| `0x90000` 起 | Loader 的过渡页表 |
| `0x100000` | Kernel 装载地址 / `_start` |
| `0x101000/102000/103000` | 内核 PML4/PDPT/PDE，head.S 中固定 `.org` |

E820 原缓冲区 `0x7e00` 在条目较多时会与 VBE 的 `0x8000` 重叠。
迁移改用 `0x5000`，显式传数量，检查 SMAP 签名、条目长度和容量，失败时停在 Loader。过渡页表的 3 个 4 KiB 页也先完整清零，再写表项，
不能假定 BIOS 已清零或只写 64 位表项的低 32 位。
任何改动必须同时更新 Loader 与 `memory.rs`，不能只改消费者。

内核链接虚拟基址是 `0xffff800000100000`（高半区偏移加 1 MiB）。
head.S 初始直映物理 `[0,10 MiB)`，虚拟偏移 `+0xffff800000000000`；
虚拟偏移 `[10,26 MiB)` 是 VBE 显存窗口，PDE 按选中模式的物理帧缓冲地址填写。
这不是全部物理 RAM 的映射。页表与 GDT 初始描述不设置用户访问权限。

head.S 切换到高半区后清零完整 BSS，设置 64 KiB 内核栈，16 字节对齐后 CALL Rust。
链接脚本同时收集普通及 large code model 的 `.ltext/.lrodata/.ldata/.lbss`，
否则 Rust 的静态对象可能落到 `_end` 之外，逃过 BSS 清零并与元数据重叠。
平坦 `kernel.bin` 不含 NOBITS，必须由入口清零；`bin/system` 保留符号用于调试。

Bootloader 不必整体用汇编。Rust [bootloader crate](https://docs.rs/bootloader/latest/bootloader/)
已有 BIOS/UEFI 路径；[x86_64](https://docs.rs/x86_64/latest/x86_64/instructions/index.html)
提供端口、描述符表、TLB 等指令的薄封装。本次保留教材 BIOS/FAT12 和模式切换，
只迁移现有 C 内核；16 位实模式入口并不是普通 x86_64 Rust 目标能直接替代的。
当前薄封装放在本地 `arch.rs`，便于学习且不引入下载依赖。
后续如改为 UEFI 或成熟 Rust 引导器，需要单独确定学习目标和新的交接协议。
