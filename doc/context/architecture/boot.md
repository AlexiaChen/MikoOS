# BIOS/FAT12 启动与 Rust 边界

源码：`src/boot/{boot.asm,fat12.inc,loader.asm}`、`src/kernel/{head.S,kernel.lds}`、
`main.rs` 的 `global_asm!`。

第 6 章的模式转换、段缓存、地址变换与 Rust 启动状态检查见 [处理器体系结构](processor.md)。

## 第 7 章第一部分：实模式寻址与搬运

按第 7 章目录从“实模式的寻址瓶颈”开始。原资料此小节只有标题，
代码参考为 `c06/6-1`、`c07/7-2/物理平台` 的 Loader 搬运路径。
`code/c07/7-1` 实际是独立 U 盘实验，其 Loader 只显示文字，不是完整内核引导实现。

每个 512 字节扇区先由 BIOS INT 13h 读取到 `0000:7e00`，
再由 `_CopyKernelSector` 完成一次受控模式往返：

```text
实模式：保存 EFLAGS/GPR/DS/ES → CLI → LGDT → PE=1
  → 远跳转 CODE32：平坦 DS/ES → A20 检查 → REP MOVSB 复制 512 字节
  → 远跳转 CODE16（D=0）：PE=0 → 远跳转 1000:返回偏移
  → 恢复 DS/ES/GPR/EFLAGS → 实模式读取下个扇区
```

目标使用 32 位 EDI，从物理 1 MiB 连续推进，跨 64 KiB 不会因 DI 溢出回绕。
保护模式窗口不调用 BIOS、不访问栈，始终 IF=0；返回代码段基址 0、界限 4 GiB、D=0，
允许在 Loader 物理位置执行清 PE 和实模式远跳转。
实模式 CS=0x1000，返回偏移是 `label - LoaderPhysicalBase`，不能直接使用物理地址。
现有 x86_64 Rust 目标不能直接生成此处的 16/32 位过渡，故硬件边界保留汇编。

原路径先加载 FS 的 4 GiB 隐藏缓存，随后调用 BIOS，并在复制前于实模式再次加载 FS。
段缓存受重载与固件操作影响，不能把本机 Bochs 的行为推广到所有 BIOS。
新路径每次复制都显式加载 GDT/平坦段，返回时恢复实模式现场，不依赖 FS 跨 BIOS 保持状态。
这是相对原版 unreal mode 搬运方式的本地取舍。

A20 通过端口 0x92 请求开启，同时清 bit0，避免误触发快速复位。
每次复制前保存物理 `0x500`/`0x100500` 的两个字节，分别写 0/0xff，
核对低地址仍为 0，然后先恢复高字节、再恢复低字节；关闭 A20 导致别名时也能恢复原数据。
失败则返回实模式，向模拟器 debugcon 输出 `Loader ERROR: A20 address alias` 后停机。
探测依赖既有低端 RAM 与 1 MiB 起内核 RAM 的引导假定，不是通用物理内存探测。

复制完成输出 `Loader copy OK: A20 verified; protected-mode sector copy`。
smoke 另在 head.S 第一条指令前停在恒等映射的 `0x100000`，用 Bochs `writemem`
导出恰好 `len(kernel.bin)` 字节，与构建文件逐字节比较，再继续执行内核。
不能在 head.S 修改页表/GDT/BSS 之后才比较初始镜像。

本部分面向自生成 FAT12 软盘。尚未验证 A20 失败注入、损坏 FAT 链、
NMI 打断过渡窗口或真机；未增加其他 A20 开启途径或 U 盘/硬盘 EDD。
后续继续 E820、引导布局、U 盘与 VBE，不能把本部分视为整章完成。

## 交接与内存布局

第 7 章 E820 采集、v1 交接头与 Rust 范围校验见 [E820 交接](bootinfo.md)。

| 物理地址 | 用途 |
| --- | --- |
| `0x4ff0..0x5000` | E820 v1 头：magic/version/stride/count/capacity/complete |
| `0x5000` | 最多 32 个 E820 条目，每条 20 字节 |
| `0x5300..0x5318` | BIOS E820 24 字节 scratch |
| `0x6000..0x7c00` | 约定的早期栈空间，SP 从 0x7c00 向下增长，无硬件 guard |
| `0x7c00` | BIOS 引导扇区/早期栈区域 |
| `0x7e00` | Loader 读内核时的单扇区临时区 |
| `0x8000` | VBE ControllerInfoBlock |
| `0x8200` | 最终选定 `0x180` 模式的 VBE ModeInfoBlock |
| `0x8200..0x10000` | VBE 枚举临时区；写入前限界，最后覆盖 0x8200 为选定模式 |
| `0x10000` | Loader 装载地址 |
| `0x90000` 起 | Loader 的过渡页表 |
| `0x100000` | Kernel 装载地址 / `_start` |
| `0x101000/102000/103000` | 内核 PML4/PDPT/PDE，head.S 中固定 `.org` |

E820 原缓冲区 `0x7e00` 在条目较多时会与 VBE 的 `0x8000` 重叠。
迁移改用 `0x5000`；第 7 章进一步以完整标志与版本头交接，检查 SMAP、20/24 字节长度和容量，失败时停在 Loader。过渡页表的 3 个 4 KiB 页也先完整清零，再写表项，
不能假定 BIOS 已清零或只写 64 位表项的低 32 位。
任何改动必须同时更新 Loader、`bootinfo.rs` 与 `memory.rs`，不能只改消费者。

内核链接虚拟基址是 `0xffff800000100000`（高半区偏移加 1 MiB）。
head.S 初始直映物理 `[0,10 MiB)`，虚拟偏移 `+0xffff800000000000`；
虚拟偏移 `[10,26 MiB)` 是 VBE 显存窗口，PDE 按选中模式的物理帧缓冲地址填写。
这不是全部物理 RAM 的映射。初始页表为 Supervisor；GDT 另有用户段槽位，
但安装用户段本身不会开放内存。5-1 建立独立低地址用户页，见 [用户态](usermode.md)。

head.S 切换到高半区后清零完整 BSS，设置 64 KiB 内核栈，16 字节对齐后 CALL Rust。
链接脚本同时收集普通及 large code model 的 `.ltext/.lrodata/.ldata/.lbss`，
否则 Rust 的静态对象可能落到 `_end` 之外，逃过 BSS 清零并与元数据重叠。
平坦 `kernel.bin` 不含 NOBITS，必须由入口清零；`bin/system` 保留符号用于调试。
5-1 新增 `.user.text`，链接符号准确界定可复制载荷，长度不得超过一个 4 KiB 页。
5-2/5-3 的 syscall-check 将 `.user.check` 紧随 Rust 载荷收进同一输出段，
其中相对 CALL/LEA 随整段搬迁保持正确；普通载荷不能引用段外内核代码或数据。

Bootloader 不必整体用汇编。Rust [bootloader crate](https://docs.rs/bootloader/latest/bootloader/)
已有 BIOS/UEFI 路径；[x86_64](https://docs.rs/x86_64/latest/x86_64/instructions/index.html)
提供端口、描述符表、TLB 等指令的薄封装。本次保留教材 BIOS/FAT12 和模式切换，
内核功能优先 Rust；16 位实模式入口并不是普通 x86_64 Rust 目标能直接替代的。
当前薄封装放在本地 `arch.rs`，便于学习且不引入下载依赖。
后续如改为 UEFI 或成熟 Rust 引导器，需要单独确定学习目标和新的交接协议。
