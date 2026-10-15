# 用户态入口（5-1）

源码：`src/kernel/{usermode.rs,usermode.S}`。主程序在内核线程退出并回收后，
运行一次有限的用户程序演示，再继续 PS/2 键盘接收。
策略、用户载荷和诊断用 Rust；汇编只处理 SYSEXIT、特权入口和挂起调用现场的恢复。
5-2/5-3 在此载荷中加入可返回用户的 [SYSENTER 服务请求](syscall.md)，完成通知仍只负责结束演示。

## 两条必须连起来理解的主线

**CPU 特权级决定能做什么，页表决定能访问什么。** 当前内核 CS=0x08、CPL=0。
按原版路线设置 IA32_SYSENTER_CS（MSR 0x174）为 0x08，准备 RDX=用户 RIP、
RCX=用户 RSP，执行带 REX.W 的 SYSEXIT。Intel64 由此生成
CS=(0x08+32)|3=0x2b、SS=(0x08+40)|3=0x33，并在 64 位用户态运行。
SYSEXIT 的段隐藏缓存由 CPU 构造，并非从对应 GDT 描述符读取；GDT 槽 5/6
仍放置一致的用户段，槽 8/9 的 TSS 与选择子 0x40 保持原布局。
不加 REX.W 会进入 32 位兼容模式。SYSEXIT 不替软件恢复 RFLAGS，入口显式设置
IF=IOPL=DF=0，清掉未使用 GPR 的内核残留；本节用户程序有限执行，全程不开放中断。

然后才是内存权限。四级页表沿途的 U/S 都须为 User，CPL3 才能访问叶子页；
RW 也受各级共同限制。只设置用户 CS 并不自动隔离内核。
EFER.NXE 开启后，PTE bit63 才表示禁止取指；CR0.WP 使内核也遵守只读映射。
EFER.NXE/CR0.WP 在演示后保留；SYSENTER 的 CS/ESP/EIP 则由 syscall 模块配置、读回并在结束后清零。
CPU 检查要求 GenuineIntel、SEP 和 NX；Bochs 的交互/smoke 配置统一选用
`corei7_sandy_bridge_2600k`。这是本节明确的 Intel64 教学路径，不声称任意 x86-64 CPU 支持。
机制依据见 [Intel 指令参考中的 SYSEXIT](https://cdrdv2-public.intel.com/774492/325383-sdm-vol-2abcd.pdf)，
模拟 CPU 选择见 [Bochs 配置说明](https://bochs.sourceforge.io/doc/docbook/user/bochsrc.html)。

## 地址布局与所有权

内存初始化先撤掉引导用的低地址恒等别名。本节重新使用 PML4[0]，但指向独立的
静态 PDPT/PD/PT，不与 PML4[256] 的高半区共享下级表。三张表均为 BSS 内的 4 KiB 对齐页，
已由内核保留区覆盖；原有高半区 RAM/显存始终为 Supervisor。

| 用户虚拟区间 | 映射 | 权限 |
| --- | --- | --- |
| `[0x800000,0x801000)` | 分配页的前 4 KiB，Rust 载荷 | User、只读、可执行 |
| `[0x801000,0x9ff000)` | 未映射，包含栈下方 guard page | 无访问权限 |
| `[0x9ff000,0xa00000)` | 同一分配页的第二个 4 KiB | User、读写、NX |
| 其余低地址 | 未映射 | 无访问权限 |

保留原版用户入口 8 MiB、栈上界 10 MiB 的虚拟位置，但不要求背后的物理地址相同。
PD[4] 使用 4 KiB PTE，PT[0] 为代码，PT[511] 为栈。物理分配仍用已有 Mapped
接口取得整个 2 MiB 页，只使用前 8 KiB，暴露前完整清零；没有引入小页分配器。
RSP 初值为上界减 8，符合普通 C ABI 函数入口的对齐约定；载荷不执行 RET。
Rust 代码做真实 volatile 栈读写和无溢出歧义的整数运算，随后请求文字输出并核对返回值，
最后上报计算结果、检查状态及再次读取的 CS/SS。四个用户故障 feature 在服务请求前触发异常。

载荷独立放入 `.user.text`，链接符号 `__user_start/end` 限定准确长度，链接断言不超过
4 KiB。不猜测函数大小、不复制邻近代码。它不能调用高半区函数、访问内核静态数据或产生
panic 路径；当前 large/static 编译模型不会替复制后的代码重定位。修改载荷后须检查
`objdump -d -j .user.text bin/system`，普通载荷应只有局部栈访问、常量运算、RIP 相对返回标签、SYSENTER、读段和 INT。
`syscall-check` 还允许相对 CALL 到同段的 `.user.check`，链接脚本将其收在同一复制范围内。
`fault-user-kernel` 故意读取高半区；`syscall-check` 仅把高半区地址作为应拒绝的指针参数。

## 为什么需要独立返回栈

用户不能调用内核的打印函数。演示期间临时安装 IDT[0x80] 为 DPL3 中断门、IST=0；
用户执行 INT 0x80，CPU 才能按该门的内核 CS 切回 CPL0，并从 TSS.RSP0 选择内核栈。
门的 DPL 控制软件 INT 是否允许进入；TSS 给出新栈，并不替软件保存全部寄存器。

RSP0 临时指向专门的 32 KiB 完成通知栈，不能重新使用启动栈顶：启动栈上还挂着
`user_enter` 的调用链，从顶端压入新现场会覆盖它。IST1/2/3 不变，故障仍在独立异常栈诊断。
入口保存与 `InterruptFrame` 一致的现场、恢复内核段并按 C ABI 调用 Rust。
处理函数核对 CPU 保存的 CS/SS/RFLAGS/RIP/用户 RSP、实际内核 RSP、TSS.RSP0 和计算结果。

完成后从私有保存位置恢复启动 RSP 和六个 callee-saved GPR，返回原内核调用者；
不接受用户提供的内核返回地址，也不恢复用户执行。这只是本节演示的完成通知，
服务编号、用户指针和分发另由 SYSENTER 入口处理；完成门没有完整的进程退出协议。
正常清理先禁用并清空 SYSENTER MSR，再恢复 RSP0、关闭临时门、清 PML4[0] 并重载 CR3，最后释放物理页。
先撤映射再释放，防止旧 TLB 翻译访问后来被复用的页。

## 与原版的差异及范围

原版 `code/c04/4-11` 与 `c05/5-1` 的 `memory.c` 都把清低 PML4 的循环注释掉了。
5-1 又在 head.S 中将 PDPT/PDE 的 U/S 打开（例如 PDE 从 0x83 变成 0x87），
原低/高半区共享的 RAM 和显存由此可以被 CPL3 访问，用户函数得以直接调用 `color_printk`。
这是该阶段演示特权转换的简化，不等于 CPU 技术“已经过时”，也不是代码不能移植。
本项目此前撤掉恒等别名，原因是高半区接管后不再需要引导别名、可暴露低地址误用、
为用户虚拟空间腾出位置；用户隔离还需本节的页表权限，单纯撤映射不足以完成隔离。

本实现保留 SYSEXIT、8 MiB 用户入口和 10 MiB 栈上界，但只开放必要用户页。
不照搬固定复制 1024 字节、用户直接运行内核打印代码的做法；有限演示在启动任务中进行，
没有把已经完成的 init 内核线程改造成完整用户进程，也没有保留原版的无限忙等。
5-2/5-3 的系统调用 API 与第一个处理函数已接入；没有提前实现 ELF、execve、进程地址空间或调度器。

当前仍共享 CR3，高半区直映别名可由内核写入/执行，不是全内核 W^X；用户程序没有 FPU/SIMD、
抢占或并发支持。四个显式故障配置验证内核读取、代码写入、栈取指、HLT 被拒绝，
异常后直接停机，不回收仍处于活动状态的实验页。命令与实测证据见
[构建](../development/build.md) 和 [验证](../development/validation.md)。
