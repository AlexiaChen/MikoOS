# 系统调用（5-2 / 5-3）

源码：`src/kernel/{syscall.rs,syscall.S,syscall_check.S}`，用户载荷及地址检查在
`usermode.rs`。对照原版 `code/c05/5-2` 的 API 和 `5-3` 的第一个处理函数。
分发、用户内存检查和输出用 Rust；汇编只处理 CPU 入口、保存现场和特权返回。

## 请求怎样越过权限边界

用户不能直接调用内核打印函数，必须通过内核预先约定的入口请求服务。
本项目沿用教材的 Intel64 SYSENTER/SYSEXIT 路线，而非 SYSCALL/SYSRET。
在用户演示开始前，内核设置并读回三个 MSR：

| MSR | 地址 | 本项目值 |
| --- | --- | --- |
| IA32_SYSENTER_CS | 0x174 | 0x08：入口 CS，SS 为 0x10 |
| IA32_SYSENTER_ESP | 0x175 | 独立 32 KiB 内核服务栈顶 |
| IA32_SYSENTER_EIP | 0x176 | `system_call_entry` 的高半区地址 |

Intel64 中后两者容纳完整 64 位地址。执行 SYSENTER 时，CPU 转到 CPL0，
加载预置 RIP/RSP 并清 IF；它不查 IDT，也不从 TSS.RSP0 取栈。
同一块服务栈同时用作完成通知的 RSP0 栈，但两种硬件选栈途径不同。
本演示 IF=0、单核且没有嵌套请求，所以可以顺序复用；不能据此支持可抢占用户任务。
硬件语义见 [Intel 指令参考第 2B 卷的 SYSENTER/SYSEXIT](https://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-software-developer-vol-2b-manual.pdf)。

SYSENTER 不像 CALL 压返回地址，也不像中断门压 RIP/CS/RFLAGS/RSP/SS。
用户包装因此显式准备 RCX=用户 RSP、RDX=返回标签 RIP。
返回标签用 RIP 相对寻址，随整个载荷复制后仍正确；包装强制内联，避免产生高半区 CALL。
这些寄存器是用户输入，不能称为“CPU 保存的 CPL3 现场”。

入口先 CLD，保存全部 15 个 GPR，恢复内核 DS/ES 并清 FS/GS，
然后将栈按 System V ABI 对齐，调用 Rust 分发函数。
`Frame` 只有 GPR，大小 120 字节，RAX 偏移 112；不沿用中断的 176 字节结构。
分发函数检查活动状态、现场位于专用内核栈、返回 RIP 在实际复制的代码范围内、
返回 RSP 在用户栈页内。无效返回现场直接诊断并停机，不尝试特权返回。

服务返回值写入保存的 RAX，其余 GPR 恢复，包括协议输入 RCX/RDX。
SYSEXIT64 根据 RDX/RCX 恢复用户 RIP/RSP，生成 CS=0x2b、SS=0x33。
SYSEXIT 不恢复 RFLAGS，故软件明确设置为 2：IF/IOPL/DF/TF/AC/NT 和算术标志清零。
这与 IRQ 的 IRETQ 恢复原标志不同；用户包装不得假设算术标志跨调用保持。
段寄存器返回到固定用户 DS/ES=0x33、FS/GS=0，没有 TLS 或任意段状态保存。

## 当前 ABI 与第一个服务

| 项目 | 约定 |
| --- | --- |
| RAX 输入 | 调用号 |
| RDI / RSI 输入 | 参数一 / 参数二 |
| RCX / RDX 输入 | 用户返回 RSP / RIP，由包装构造 |
| RAX 输出 | 非负成功值，或有符号负错误码 |
| 其余 GPR | 保留；RCX/RDX 是包装占用的返回协议寄存器 |
| RFLAGS 输出 | 2；不保留算术标志，全程 IF=0 |

分发表沿用教材的 128 槽。1 号为 `write_text(pointer, length)`，其余槽为默认服务。
索引先 `.get` 检查边界；空槽及越界调用号都返回 `-38`（ENOSYS），
包括教材演示的 15 和表外的 128/u64::MAX。错误码采用本项目明确的约定，
不是现成的 Linux/POSIX 系统调用 ABI，也没有文件描述符。

文字服务每次最多 256 字节，成功返回实际长度；长度超限或无效 UTF-8 返回
`-22`（EINVAL）。零长度返回 0，不解引用指针，允许 NULL。
非空区间必须完整属于代码页或栈页，末地址用 checked_add 防止整数溢出；
NULL、内核地址、未映射空洞或跨出页边界返回 `-14`（EFAULT）。地址布局见
[用户态](usermode.md)。代码页只读并不妨碍作为只读服务输入。

内核逐字节复制到有界私有缓冲，不从用户指针直接构造 Rust 字符串/切片引用；
复制与 UTF-8 校验完成后才输出。用户文字只是 `kprint!(..., "{text}")` 的参数，
`%s`、`%n`、花括号不会成为内核格式串。屏幕仍使用原字体，日志保留 UTF-8 字节；
没有引入 Unicode 字形系统。普通演示输出 `Hello World! %s %n` 和换行，共 19 字节。

## 演示结束与验证边界

用户载荷核对每次调用结果；`syscall-check` 追加边界请求和纯汇编 GPR/栈/标志哨兵。
最终仍以临时 INT 0x80 完成通知上报计算结果及检查状态。
该门从 TSS.RSP0 选栈，并核对 CPU 保存的 CS/SS，证明多次系统调用后仍在 CPL3 执行。
它不分发服务，不是第二套系统调用 API，也不是完整进程 exit。
随后恢复原内核调用链，清 SYSENTER_CS 禁用入口，清 ESP/EIP 并读回，
关闭完成门，恢复 RSP0，撤用户映射、刷新 TLB 后释放物理页。

相对原版修正：不执行入口 STI、不无界索引函数表、不直接解引用用户字符串或把它作为格式串，
不以固定 1024 字节复制载荷。原版的服务返回 1，本项目返回实际输出字节数和明确错误码。

本节是共享 CR3 下的有限内置程序，不含 ELF、独立进程空间、调度、文件服务或用户 API 库。
用户指针检查依赖当前固定两页布局及 IF=0：没有并发撤映射、可恢复内核缺页或通用页表遍历。
无效返回现场拒绝分支尚未通过恶意 RCX/RDX 注入验证；实际模拟器证据见
[验证记录](../development/validation.md)，运行命令见 [构建](../development/build.md)。
