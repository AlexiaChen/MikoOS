# 验证记录

最新状态见末尾的 **2026-10-16：第 7 章 E820 与引导内存布局**；以下保留各阶段当时的验证边界。

2026-10-13：迁移基线 `a76a0b8` 的 C 内核到 Rust，未推进未实现的后续章节。
验证环境：Rust 1.97.1、x86_64-unknown-none、Bochs 2.7、legacy BIOS、2 GiB RAM。

## 已执行

- 普通、`fault-divide`、`fault-page` 三种配置的 `make check`：fmt 与 release Clippy（`-D warnings`）全部通过。
- 三种配置的 `make build`：离线构建成功，软盘 1,474,560 字节，Loader 2,301 字节，Kernel 45,288 字节。
  每次镜像均解析 BPB、检查双 FAT 和目录、沿 FAT 链重读两个文件并与输入逐字节比对。
- `python3 scripts/smoke.py`：正常路径完成 E820、内存一致性、移除低地址映射，CPU 到达 `kernel_halt` 断点。
- `python3 scripts/smoke.py --expect fault-divide`：真正触发 CPU #DE，错误码 0，进入 Rust 异常函数并到达停机断点。
- `python3 scripts/smoke.py --expect fault-page`：真正触发 CPU #PF，错误码 0，
  CR2=`0xffff80000aa00000`，只读/内核/未映射页解码正确，进入停机断点。
- 同时启用两个故障 feature：编译按预期拒绝，输出“一次只启用一个异常演示功能”。
- 原 HEAD 的 `font.h` 与当前 `font.rs` 全部 4096 字节一致。
- `nm -u bin/system` 无未解析符号；ELF64 x86-64 入口为 `0xffff800000100000`。
- Python 脚本语法、文档相对链接、`git diff --check` 通过。

最终已执行 `make build` 恢复普通镜像；SHA-256 比对确认 kernel.bin 和 boot.img 与已通过
正常启动验证的版本逐字节相同，不将故障演示镜像留为默认产物。

## 实际日志与解释

```text
OS usable RAM:0x000000007ff9f000; usable 2M pages:1023
Memory invariants OK: free=1023 reserved=1
Identity mapping removed; high-half mapping retained
MikoOS Rust kernel ready
```

可复查日志在 `bin/smoke-normal.log`、`bin/smoke-fault-divide.log`、`bin/smoke-fault-page.log`，
对应模拟器日志在 `bin/bochs-{normal,fault-divide,fault-page}.log`，均为本地生成文件，不提交。
默认机器上第 0 个 2 MiB 页包含内核与元数据，单独保留；完整 RAM Zone 从 2 MiB 开始，
因此 free=1023、reserved=1 并不意味着 Zone 漏记了一页。

## 验证边界

系统自带 `BIOS-bochs-latest` 在 rombios32 中、进入引导扇区之前产生三重异常；
改用系统自带 legacy BIOS 后可启动，交互与 smoke 配置已统一。
本机 RFB 后端报告 `32 bpp graphics mode not supported yet`，仅该组件的该报告被允许；
其余 panic 仍失败。上述结果证明 CPU/页表/异常路径以及 MMIO 写入流程成功，
不代表 GUI 可视呈现已验证。没有验证 x 显示后端、Windows、UEFI、其他模拟器或真机。
上述 2026-10-13 基线中 `clean_page` 尚无上层调用；新增分配器的验证见下节。

## 2026-10-14：连续物理页分配与释放

实现依据为父目录原版 `code/c04/4-8/kernel/memory.c` 和 `main.c`，未修改参考目录。
验证环境沿用 Rust 1.97.1、x86_64-unknown-none、Bochs 2.7、legacy BIOS、2 GiB RAM。

已实际执行：

- 普通、`memory-check`、`fault-divide`、`fault-page` 四种配置的 `make check` 均通过。
- 四种配置的 `make smoke` 均构建成功并通过 FAT12 镜像检查；CPU 均到达 `kernel_halt`。
  普通内核 49,384 字节，内存检查内核 53,552 字节，Loader 仍为 2,301 字节；镜像 1,474,560 字节。
- 普通启动各分配一个 Mapped/Any 页，确认地址不同，对 Mapped 页首尾 volatile 写入并读回，
  然后归还两页，打印 `Physical page allocation/read-write/free OK`。
- `memory-check` 覆盖零页/超大请求、65 页跨 u64 位图边界、不重叠、释放后原位复用、
  映射区耗尽及单页碎片导致的连续分配失败、完整 RAM Zone 耗尽、失败状态不变和最终恢复。
  逐页核对位图、归属、属性、引用数和 Zone 使用/空闲/引用计数，保留页始终为已用。
- #DE 仍为向量 0、错误码 0；#PF 仍为向量 14、错误码 0，CR2=`0xffff80000aa00000`。
- 最后执行普通 `make smoke`，恢复普通镜像并重新确认正常启动，不留故障或扩展检查为默认产物。
- Python 脚本语法、文档相对链接、四份启动日志、`git diff --check` 通过；
  `nm -u bin/system` 无未解析符号，ELF64 x86-64 入口仍为 `0xffff800000100000`。

扩展内存检查的关键输出：

```text
Physical page allocation/read-write/free OK
Page allocator checks OK: cross-word, reuse, fragmentation, exhaustion; free=1023
MikoOS Rust kernel ready
```

记录在 `bin/smoke-memory-check.log`、`bin/bochs-memory-check.log`，正常与故障日志保持独立；均不提交。
首次沙箱内 smoke 因 RFB 无法创建本地套接字而失败，允许模拟器运行后四条路径通过；不是内核故障。

限制：扩展验证明确要求当前单个无预留 RAM Zone、至少 65 个连续空闲页和两个映射区空闲页。
未验证多 Zone/无序 E820/大内存机器的运行分配，不能把本次结果推广到任意 BIOS 布局。
高地址分配只验证记账，未映射 RAM 不作实际读写；普通读写验证只覆盖所分配映射页的首尾。
没有堆、共享页、并发分配、动态页表或性能验证；GUI 与真机限制仍同上。

## 2026-10-14：第 4 章 4-9 外部中断

上一阶段物理页分配及文档已创建本地提交 `cf17d5e`，随后实现本节；未修改父目录参考项目。
环境仍为 Rust 1.97.1、Bochs 2.7、legacy BIOS、2 GiB RAM。

已实际执行：

- 普通、`irq-check`、`memory-check`、`fault-divide`、`fault-page` 五种 `make check` 均通过。
- 五种 `make smoke` 均完成离线编译、FAT12 镜像检查，并到达真实 `kernel_halt` 断点。
  普通和两种故障内核为 49,384 字节，IRQ 检查内核 53,480 字节，内存检查内核 57,648 字节；
  Loader 2,301 字节，软盘镜像 1,474,560 字节。
- 普通启动：安装 PIC 16 个门，读回两片 IMR=0xff 和空 ISR；只开放 IRQ0。
  收到两次来自 PIC 的 PIT IRQ0，处理函数每次确认 ISR0=1，发送 specific EOI 后确认 ISR0=0；
  CPU 经 IRETQ 返回主流程，演示结束 PIC 全屏蔽、IF=0，然后到达最终停机。
- `irq-check`：软件 INT 0x27 前在 15 个 GPR 放入不同哨兵，并设 DF 与算术标志；
  返回后逐一核对 GPR、RSP 和 CF/PF/AF/ZF/SF/IF/DF/OF（此检查的原 IF=0）。
  软件 INT 0x2f 覆盖伪从片中断分支；两种伪中断计数各为 1，ISR 和硬件时钟计数仍为 0。
  随后的两次真实 PIT IRQ0 独立通过。
- 内存检查仍恢复到 1023 个空闲页；#DE 仍为向量 0、错误码 0；
  #PF 仍为向量 14、错误码 0、CR2=`0xffff80000aa00000`。
- 最后执行普通 `make smoke`，恢复并重新验证默认镜像，未留下检查/故障配置。
- Python 语法、文档链接、五份已保存日志与 `git diff --check` 均通过；
  最终 ELF 无未解析符号，入口保持 `0xffff800000100000`。

关键输出：

```text
8259A ready: vectors=0x20..0x2f, all IRQs masked
IRQ context checks OK: GPRs, RFLAGS, RSP; software IRQ7/15 spurious paths
PIT IRQ0 delivered twice; PIC EOI and IRETQ return OK
MikoOS Rust kernel ready
```

其中第二行只在 irq-check 下出现。证据位于 `bin/smoke-irq-check.log` 与
`bin/bochs-irq-check.log`；其余四种日志分别保存，均不提交。
初次沙箱内启动仍因 RFB 本地套接字受限失败，允许模拟器运行后通过；已知 32 bpp 限制仍仅按白名单容忍。

新增 IRQ IST3 栈为 32 KiB。普通配置 `_end=0xffff800000133000`，
元数据末端 `0xffff800000148a00`，仍完全位于已保留的首个 2 MiB 页内。

边界：真实硬件交付只覆盖主片 IRQ0；从片 EOI 路径尚未用真实从片设备验证。
伪中断只通过软件 INT 模拟 ISR 为空的处理分支，未验证电气级伪请求。
未做嵌套中断、SMP、APIC、IRQ 内存分配、SIMD/FPU 状态保存、键盘/RTC/磁盘中断、
时间精度或持续运行性能验证。无桌面 smoke 不证明 GUI 呈现，亦未做真机验证。

## 2026-10-14：第 4 章 4-10 键盘中断

上一阶段外部中断已创建本地提交 `2774174`。随后对照父目录原版 4-10 的 IRQ1/0x60 实验，
实现 PS/2 扫描码捕获；没有修改参考项目。环境沿用 Rust 1.97.1、Bochs 2.7、legacy BIOS、2 GiB RAM。

已实际执行：

- 普通、keyboard-check、irq-check、memory-check、fault-divide、fault-page 六种 `make check` 均通过。
- 六种 `make smoke` 均成功构建并通过 FAT12 检查。普通/IRQ/故障内核 53,488 字节，
  keyboard-check 内核 57,632 字节，memory-check 内核 61,744 字节；Loader 2,301 字节，镜像 1,474,560 字节。
- 普通、irq-check、memory-check 在 `kernel_keyboard_wait` 断点退出，证明初始化进入接收边界。
  新增 8042 有限轮询、旧输出排空、PS/2 ACK、扫描集配置与 IRQ1 开放流程完成。
- keyboard-check：局部队列验证 FIFO、环绕和满时丢新字节；宿主从本次 Bochs 日志发现 RFB 端口，
  完成无认证握手，通过 KeyEvent 依次发送 a 按下/松开、b 按下/松开。
  内核收到 1e、9e、30、b0，IRQ 分发每次确认 ISR1 在服务，消费 8042 字节后 EOI 并确认 ISR1 清零；
  队列零残留、零丢失、零错误，随后全屏蔽并到达 `kernel_halt`。
- IRQ GPR/RFLAGS/RSP 恢复仍通过；物理页检查恢复到 1023 个空闲页；
  #DE/#PF 向量与错误码仍正确，#PF CR2 仍为 `0xffff80000aa00000`。
- 最后执行普通 `make smoke`，恢复并验证默认交互镜像；普通内核现为持续等待输入，
  自动检查到等待入口后退出模拟器，不再把普通内核描述为初始化后最终停机。
- Python 语法、文档链接、六份保存日志、精确输入序列与 `git diff --check` 通过；
  当前 ELF 无未解析符号，入口仍为 `0xffff800000100000`，普通日志断点与当前等待符号一致。

关键输入输出（`bin/smoke-keyboard-check.log`）：

```text
Keyboard queue checks OK: wraparound, FIFO, overflow
PS/2 keyboard ready: IRQ1, translated set 1 scan bytes
MikoOS Rust kernel ready
Keyboard scan byte: 0x1e
Keyboard scan byte: 0x9e
Keyboard scan byte: 0x30
Keyboard scan byte: 0xb0
Keyboard IRQ1 checks OK: a make/break, b make/break; no drops/errors
```

模拟器证据在 `bin/bochs-keyboard-check.log`，其余五种日志独立保存；均不提交。
初次普通 smoke 仍因沙箱禁止 RFB 创建本地套接字而失败，允许模拟器运行后上述检查通过。

普通配置 `_end=0xffff800000134000`、元数据末端 `0xffff800000149a00`，
仍完全在保留的首个 2 MiB 页内。VNC 只发送输入，不请求画面更新；未验证 GUI 可视呈现。
实际输入只覆盖模拟 PS/2 的 a/b 按下与松开；未验证扩展键、组合键、字符转换、真实队列满、
ACK 超时/RESEND、坏数据、辅助端口、初始化时按键干扰、USB/真机或其他 BIOS/控制器。
普通驱动不解释扫描码序列，缺失字节后的解析恢复和更完善的设备协议留给后续章节。

## 2026-10-14：第 4 章 4-11 初始内核线程

4-10 已创建本地提交 `454e801`，随后参考原版 `code/c04/4-11/kernel/{task.c,task.h,main.c}`
实现首个内核线程；辅助核对原版 lib.h 与 Kernel.lds，没有修改父目录。
环境仍为 Rust 1.97.1、Bochs 2.7、legacy BIOS、2 GiB RAM。

已实际执行：

- 普通、task-check、keyboard-check、irq-check、memory-check、fault-divide、fault-page 七种 `make check` 通过。
- 七种 `make smoke` 构建/镜像检查通过。普通/task-check/irq-check/故障内核 61,768 字节，
  keyboard-check 65,864 字节，memory-check 69,976 字节；Loader 2,301 字节，软盘 1,474,560 字节。
- 普通演示在物理 2 MiB 页创建 pid1，描述符位于 `0xffff800000200000`，
  栈范围 `0xffff800000201008..0xffff800000208000`；init 在独立栈上接收参数 10、让出、恢复、返回 1。
  启动任务两次恢复均核对状态/进度，退出后确认 Zombie/退出码及边界哨兵完整。
- 每次恢复核对实际 CPU RSP 在当前栈范围内、TSS.RSP0 读回和共享 CR3 正确；
  返回启动栈后归还整页，重新申请命中同一物理页并再次归还，确认可复用。
- task-check 为两个任务生成不同的 RBP/RBX/R12..R15 哨兵，核对三次完成返回的六个 GPR、
  RSP 和栈局部哨兵；init 独立栈上软件 INT 0x27 通过 IST3/IRETQ 回到原任务栈。
- keyboard-check 在任务演示/回收后仍经宿主 VNC 收到 1e/9e/30/b0，零丢失/错误；
  IRQ 现场恢复仍通过，memory-check 仍恢复到 1023 个空闲页，#DE/#PF 结果与 CR2 保持正确。
- 最后执行普通 `make smoke`，恢复并验证普通交互镜像，到 `kernel_keyboard_wait` 断点退出。
- Python 语法、文档链接、七份日志中的任务执行顺序、精确键盘输入序列和 `git diff --check` 通过；
  当前 ELF 无未解析符号，入口仍为 `0xffff800000100000`，普通日志断点匹配当前等待符号。

task-check 的关键输出：

```text
Kernel thread created: pid=1, stack=0xffff800000201008..0xffff800000208000
init task is running, arg=0x000000000000000a
Bootstrap task resumed: init yielded
init task resumed after yielding
Kernel thread exit: pid=1 code=1
Task context checks OK: callee-saved GPRs, RSP, stack canaries; child IST return
Kernel thread demo OK: yield, resume, exit=1; page reclaimed
PS/2 keyboard ready: IRQ1, translated set 1 scan bytes
```

证据保存在 `bin/smoke-task-check.log`、`bin/bochs-task-check.log`；其余六种路径也独立保存，不提交。
首次沙箱内 Bochs 仍因 RFB 不能创建本地套接字失败，允许模拟器运行后通过。
普通 `_end=0xffff800000136000`、元数据末端 `0xffff80000014ba00`，仍在已保留的首个 2 MiB 页内。

边界：只有两个 ring0 执行流的显式主动交接，不是抢占调度或一般多线程压力验证。
TSS 只核对内存中的 RSP0，没有执行/验证 ring3 特权转换；子栈 IRQ 测试是软件 INT，不是硬件抢占。
未覆盖 SMP、NMI 读取任务状态、独立地址空间、FS/GS/TLS、SIMD/FPU、栈溢出注入或保护返回入口。
边界哨兵不是 guard page；只使用 2 MiB 页前 32 KiB，没有小块分配器。GUI 呈现和真机仍未验证。

## 2026-10-15：5-1 用户态入口

基线为 `b762bd5`，参考父目录 `code/c05/5-1/kernel/{task.c,entry.S,head.S,memory.c}`，未修改原版。
环境：Rust 1.97.1、x86_64-unknown-none、Bochs 2.7、legacy BIOS、2 GiB RAM；
本节明确选择 Intel `corei7_sandy_bridge_2600k` 模型，交互与 smoke 配置同步。

已实际执行：

- 普通、memory-check、irq-check、keyboard-check、task-check、fault-divide、fault-page，
  以及四种 fault-user-*，共 11 种 `make check`，fmt/release Clippy（`-D warnings`）均通过。
- 11 种 `make smoke` 均离线构建、完成 FAT12 镜像检查并到达对应 CPU 断点。
  普通/task-check/六种故障内核 69,960 字节；keyboard-check/irq-check 74,056 字节；
  memory-check 78,176 字节。Loader 2,301 字节，软盘镜像 1,474,560 字节。
- 普通路径：SYSEXIT64 后在低地址执行 Rust 载荷，真实栈读写计算成功；CPU 保存的
  CS=0x2b、SS=0x33、RIP/RSP 均为用户地址，IF/IOPL/DF=0。INT 0x80 通过 IST=0 的门
  从 TSS.RSP0 选择专用内核栈，Rust 处理函数确认自身 RSP 在该栈内、TSS 读回和结果正确。
  返回启动调用链后确认 CS=8、IF=0、栈边界哨兵完整，恢复 RSP0/IDT，清低 PML4、刷新 TLB 后释放页。
- fault-user-kernel：#PF(5)，CR2=0xffff800000100000，present/user=true、write/fetch=false。
- fault-user-write：#PF(7)，CR2=0x800000，present/write/user=true、fetch=false。
- fault-user-exec：#PF(0x15)，CR2=RIP=0x9ff000，present/user/fetch=true、write=false。
- fault-user-privileged：用户 HLT 产生 #GP(0)。四种故障均由 CPU 保存的 CPL3 现场复核，
  打印对应保护检查成功标记后到达 kernel_halt；普通路径没有主动异常。
- task-check 的三次 callee-saved GPR/RSP/哨兵恢复和子线程 IST 返回仍通过；
  irq-check 的 IRQ 全寄存器/RFLAGS/RSP 恢复和伪 IRQ7/15 分支仍通过。
  memory-check 仍恢复到 1023 个空闲页。keyboard-check 在用户演示清理后，经宿主 VNC
  实际收到 1e/9e/30/b0，零丢失/错误。原 #DE/#PF 的向量、错误码和 CR2 均保持正确。
- `cargo check --release --offline --features ...` 验证三组冲突组合均拒绝编译，且包含
  “一次只启用一个异常演示功能”：fault-divide+fault-page、fault-user-write+fault-user-exec、
  fault-page+fault-user-kernel。
- 检查普通与用户页故障配置的 `.user.text` 反汇编，无跨段调用；普通载荷为 58 字节，
  只有局部栈访问、立即数运算、读段选择子和 INT，没有高半区引用。
  故障内核读取载荷仅为实验故意包含内核地址；写入/取指目标都是指定的用户地址。
- 最后重新执行普通 `make smoke`，恢复默认交互镜像，并到 kernel_keyboard_wait 退出。
  最终 ELF 无未解析符号，入口仍为 0xffff800000100000；普通 `_end=0xffff800000144000`，
  内存元数据末端 0xffff800000159a00，仍完全位于首个已保留的 2 MiB 页。

普通关键日志：

```text
User mapping ready: code=0x800000 R-X, stack=0x9ff000 RW-NX; kernel supervisor
User mode OK: CS=0x2b SS=0x33 CPL=3, Rust stack/result verified; INT 0x80 used TSS.RSP0
User demo cleanup OK: ring0 restored, low mapping removed, page reclaimed
PS/2 keyboard ready: IRQ1, translated set 1 scan bytes
MikoOS Rust kernel ready
```

日志分别为 `bin/smoke-{normal,feature}.log` 与 `bin/bochs-{normal,feature}.log`，均不提交。
首次沙箱内运行因 RFB 套接字受限而失败；允许模拟器运行后得到上述结果。
原 generic CPU 配置报告 AuthenticAMD，被新增 CPU 前置检查明确拒绝；随后改为预定义 Intel 模型，
不是通过伪造 vendor 字符串跳过检查。RFB 的已知 32 bpp 限制仍仅按白名单容忍。

边界：本节只验证内置有限 Rust 载荷和指定的四种硬件保护，不代表完整进程安全或任意用户二进制支持。
INT 0x80 是临时完成通知，没有通用系统调用/SYSENTER 服务。共享 CR3、只映射两个用户页，
无 ELF、独立进程空间、用户抢占、FPU/SIMD、SMP 或完整内核 W^X；默认演示 IF=0。
没有验证其他 CPU 厂商、Windows、UEFI、GUI 呈现或真机。


## 2026-10-15：5-2 / 5-3 系统调用

基线为用户已提交的 `d86aa21`，对照父目录 `code/c05/5-2`、`5-3` 的
`kernel/{entry.S,task.c,task.h}`，未修改参考项目。环境沿用 Rust 1.97.1、
x86_64-unknown-none、Bochs 2.7、Intel Sandy Bridge 模型、legacy BIOS、2 GiB RAM。
本次按约定一起完成调用 API 与第一个文字服务，机制及原版差异见 [系统调用](../architecture/syscall.md)。

已实际执行：

- 普通、memory-check、irq-check、keyboard-check、task-check、syscall-check、fault-divide、
  fault-page 及四个 fault-user-*，共 12 种 `make check` 均通过 fmt/release Clippy（`-D warnings`）。
- 12 种 `make smoke` 均离线构建、通过 FAT12 镜像检查并到达对应 CPU 断点。
  普通/原 #DE/#PF 内核 69,960 字节，syscall-check 74,056 字节，
  task-check/irq-check/keyboard-check/四个用户故障内核 78,152 字节，
  memory-check 82,264 字节；Loader 2,301 字节，软盘 1,474,560 字节。
- 普通载荷真实从 CPL3 执行两次未知调用（15/128），均收到 -38；1 号服务读取用户栈中的
  `Hello World! %s %n` 及换行，原样输出 19 字节并返回 19。用户代码继续运行，核对结果，
  再执行 INT 0x80；完成通知核对 CPU 保存的 CS=0x2b、SS=0x33 和检查状态为零。
- SYSENTER 的三个 MSR 均用完整 64 位值设置并读回；服务现场在独立内核栈，返回 RIP 在
  实际复制代码内、RSP 在用户栈页内。演示退出后清 CS/ESP/EIP 并读回，恢复 RSP0/IDT，
  确认栈底哨兵和 CPL0，撤低地址映射、失效 TLB 后归还整页。
- syscall-check 追加 0/127/u64::MAX 调用号，NULL、内核地址、未映射空洞、
  栈下边界跨页、栈上边界跨页和末地址溢出的缓冲区请求；均得到预期错误码。
  零长度 NULL 返回 0；完整 256 字节栈缓冲输出成功；257/u64::MAX 长度与坏 UTF-8 返回 -22。
- syscall-check 的用户汇编向所有非协议 GPR 写入不同哨兵，同时准备 RCX/RDX，
  设置 DF/算术标志后真实执行 SYSENTER。返回时 RAX=-38，其余 GPR、RCX/RDX、
  RSP/栈哨兵保持，RFLAGS=2；随后正常返回 Rust，完成通知确认检查状态为零。
- 四种用户保护仍正确：内核读取 #PF(5)、代码写入 #PF(7)、栈取指 #PF(0x15)、HLT #GP(0)；
  CR2 与 CPL3 现场仍对应指定操作。故障实验在用户服务请求前执行，普通启动无主动异常。
- task-check 与 irq-check 的现场恢复仍通过，memory-check 仍恢复到 1023 个空闲页；
  keyboard-check 在用户服务结束/清理后，经宿主 VNC 实际收到 1e/9e/30/b0，零丢失/错误。
  原 #DE/#PF 向量、错误码和 #PF CR2 保持正确。
- 检查普通与 syscall-check 的 `.user.text` 反汇编。普通载荷 193 字节，无 CALL、
  段外数据引用或内核地址；检查载荷 1,141 字节，唯一 CALL 相对指向同复制段的
  user_syscall_check，所有 RIP 相对引用均在段内；高半区常量仅作为非法指针参数。
  两种 ELF 无未解析符号。链接时整个载荷仍断言不超过 4 KiB。
- Python 语法、文档链接、12 份保存日志、精确键盘序列和 `git diff --check` 均通过。
  最后恢复普通镜像并验证到 kernel_keyboard_wait，入口仍为 0xffff800000100000；
  普通 `_end=0xffff800000144000`，元数据末端 0xffff800000159a00，仍在已保留的首个 2 MiB 页内。

新增关键输出：

```text
SYSENTER ready: CS=0x8, dedicated kernel stack, 128 services
Hello World! %s %n
System call demo OK: SYSENTER/SYSEXIT64 returned to ring3; write=19, ENOSYS=-38
System call checks OK: GPRs, RSP, flags; number bounds, user ranges, length, UTF-8
User mode OK: CS=0x2b SS=0x33 CPL=3, Rust stack/result verified; INT 0x80 used TSS.RSP0
User demo cleanup OK: ring0 restored, low mapping removed, page reclaimed
```

第四行只出现在 syscall-check；该配置还输出 256 字节空格/换行，用于验证长度上界。
证据在 bin/smoke-{normal,feature}.log 与 bin/bochs-{normal,feature}.log，不提交。
首次沙箱运行因 RFB 无法创建本地套接字失败，获准在沙箱外运行模拟器后得到上述结果。
已知 RFB 32 bpp 限制仍只按原白名单容忍。

边界：没有注入恶意 RCX/RDX 验证无效返回现场的停机分支；没有通用用户缺页恢复、
并发撤映射、调度/抢占、独立进程空间、TLS、FPU/SIMD 或文件/进程系统调用验证。
本节仅提供共享 CR3、IF=0 下的内置载荷与一个文字服务，不是通用应用运行环境。
无桌面 smoke 不证明 GUI 字形呈现，也没有其他 CPU 厂商、Windows、UEFI 或真机验证。

## 2026-10-16：第 6 章处理器体系结构

基线为 `4b2b18e`。首先重新核对原版第 5 章三个小节与当前调用链，
普通、syscall-check、四种 fault-user 配置的 `make check` 与 `make smoke` 均通过。
章节目标已具备；独立进程/ELF/通用 execve 不在这三个小节的已实现范围内。
无效 RCX/RDX 拒绝分支的既有验证缺口仍按上一节保留，没有因本次复核而声称补齐。

然后对比 `code/c05/5-3` 与 `code/c06/6-1`：内核功能相同，入口汇编差异仅为空白，
另有 Bochs 配置差异。因此本章围绕现有模式转换补充教学说明，
并增加 Rust 只读启动状态检查，详见 [处理器体系结构](../architecture/processor.md)。
环境仍为 Rust 1.97.1、x86_64-unknown-none、Bochs 2.7、Intel Sandy Bridge 模型、
legacy BIOS、2 GiB RAM。

第 6 章实现完成后实际执行：

- 全部 12 种配置的 `make check`（fmt、release Clippy、warnings 当错误）均通过：
  普通、memory-check、irq-check、keyboard-check、task-check、syscall-check、
  fault-divide、fault-page 和四种 fault-user。
- 同一组 12 种 `make smoke` 均完成离线构建、FAT12 镜像检查及 Bochs 启动检查；
  所有配置都包含新的 `Processor mode OK` 标记，并到达各自的既有结束断点。
  最后执行普通配置，恢复普通镜像，到达 kernel_keyboard_wait。
- 真实读取 CR0.PE/PG、CR4.PAE/LA57、EFER.LME/LMA、CS 与 LAR 描述符访问权，
  核对四级分页、IA-32e 激活、64 位 ring0 代码段、物理 CR3=0x101000。
  从 RIP 相对标签与 RSP 寄存器取得真实地址，核对高半区和 48 位规范地址条件。
- 系统调用往返和边界检查、四种用户保护故障、IRQ/任务现场恢复、分配器状态恢复、
  真实键盘 IRQ1 的 a/b make/break 仍通过。普通配置没有主动 CPU 异常。
- 普通内核为 78,152 字节，Loader 为 2,301 字节，软盘为 1,474,560 字节。
  将基线 Loader 源码重新汇编后逐字节比较，确认本章 Loader 注释未改变其机器码。
- `nm -u bin/system` 无未解析符号，ELF64 入口保持 0xffff800000100000。
  检查普通 `.user.text` 反汇编，193 字节载荷无段外 CALL 或数据引用。
  Python 脚本语法、Markdown 本地链接和 `git diff --check` 均通过。

最终普通日志中的新增输出：

```text
Processor registers: CR0=0x00000000e0000011 CR3=0x0000000000101000 CR4=0x0000000000000020
Processor code: EFER=0x0000000000000500 CS=0x08 LAR=0x00209900 RIP=0xffff800000109f2c RSP=0xffff800000123ef0
Processor mode OK: IA-32e active, 64-bit code, CPL0; 4-level paging, canonical RIP/RSP
```

日志在 `bin/smoke-{normal,feature}.log` 与 `bin/bochs-{normal,feature}.log`，不提交。
LAR 输出为本次读数；检查只使用有定义且相关的访问权位，不把其余位作为固定协议值。
本次仅验证当前模式契约的成功路径，没有注入错误控制寄存器或错误 GDT，
也没有逐条截获实模式/保护模式的中间现场。此检查发生在 Rust 入口之后，
不能替代启动前的能力检查或完整页表验证。
RFB 的 32 bpp 显示限制仍在；只验证 CPU 执行、MMIO 流程与日志，没有 GUI 或真机验证。

## 2026-10-16：第 7 章第一部分

基线为第 6 章提交 `625d6b9`。本次只完成目录中的“实模式的寻址瓶颈”对应搬运完善，
不标记原版 `code/c07/7-1` U 盘实验或整个第 7 章为完成。
对照 `c06/6-1`、`c07/7-2/物理平台` 的高地址搬运方式，
以受控保护模式复制替代依赖 FS 隐藏缓存的路径，机制与差异见 [启动](../architecture/boot.md)。
环境仍为 Rust 1.97.1、Bochs 2.7、Intel Sandy Bridge、legacy BIOS、2 GiB RAM。

实现后实际执行：

- 普通 `make check`：fmt 与 release Clippy（warnings 当错误）通过。
- 全部 12 种 `make smoke` 通过：普通、memory-check、irq-check、keyboard-check、
  task-check、syscall-check、fault-divide、fault-page 和四种 fault-user。
  每次均离线编译并检查 FAT12 镜像；本次没有重新运行其余 11 种配置的 Clippy。
- 每次在 head.S 第一条指令前停住，`writemem` 导出物理 1 MiB 起的实际装载内容，
  与对应 `kernel.bin` 逐字节一致。普通为 78,152 字节，memory-check 为 82,264 字节；
  均跨过 64 KiB，覆盖实际扇区读取、累计目标偏移和复制路径，而非只凭进入 Rust 推断成功。
- Loader 每个复制窗口都通过 A20 双地址检查，全部复制完成后输出阶段标记。
  内核依次通过第 6 章模式检查、内存、IRQ、任务、用户态/系统调用与键盘路径。
  四种用户保护和原 #DE/#PF 的预期故障仍成立；普通路径没有主动 CPU 异常。
- 最后执行普通 `make smoke`，恢复普通镜像并到达 kernel_keyboard_wait。
  Loader 为 2,506 字节，普通 Kernel 为 78,152 字节，软盘为 1,474,560 字节。
- Python 语法、上下文 Markdown 链接、`git diff --check` 通过；
  `nm -u bin/system` 无未解析符号，ELF 入口保持 0xffff800000100000。

关键证据：

```text
Loader copy OK: A20 verified; protected-mode sector copy
Processor mode OK: IA-32e active, 64-bit code, CPL0; 4-level paging, canonical RIP/RSP
Loader memory verified: 78152 bytes at physical 0x100000
PASS: Bochs normal boot reached kernel_keyboard_wait
```

每份 `bin/smoke-{normal,feature}.log` 保留装载取样断点和宿主字节核对结果；
`bin/kernel-loaded.bin` 为最后一次运行的原始取样，全部位于忽略的 bin/ 内。
第一个断点用于装载取样、第二个用于原内核结束边界，历史单断点日志不适用于当前脚本。

边界：没有注入 A20 关闭验证拒绝分支，也没有损坏 FAT 链、NMI 窗口、
其他 A20 控制方式、U 盘/硬盘 EDD、其他 BIOS、GUI 或真机验证。
当前 32 bpp RFB 白名单与显示限制仍沿用前章，不将模拟器检查解释为真机支持。

## 2026-10-16：第 7 章 E820 与引导内存布局

基线为 `87442e4`。对应目录中的物理地址空间信息和引导内存划分，
参考原版 `c07/7-2/物理平台/bootloader/loader.asm` 的 E820 路径；
当前以独立 scratch、完整版本头和 Rust 检查完善交接。协议见 [E820 交接](../architecture/bootinfo.md)。

实现后实际执行：

- 全部 12 种配置的 `make check` 均通过（fmt、release Clippy、warnings 当错误），
  配置为普通、memory-check、irq-check、keyboard-check、task-check、syscall-check、
  fault-divide、fault-page 及四种 fault-user。
- 同组 12 种 `make smoke` 全部通过，包含离线构建、FAT12 镜像检查、内核装载字节比对、
  新 E820 原始交接区取样与逐条内核解析比对，随后仍到达各自既有结束断点。
  最后恢复普通配置并到达 kernel_keyboard_wait。
- Bochs 实际交接头为 magic=E820、version=1、stride=20、count=5、capacity=32、flags=1。
  五条描述符与内核输出完全一致，新增固定布局检查通过；普通页数仍 free=1023、reserved=1。
- memory-check 的局部输入验证通过：32 条数量上界可接受；坏 magic/version/stride/capacity、
  incomplete、0/33 数量被拒绝；端地址溢出、重复 RAM、RAM 覆盖 reserved 被拒绝；
  无序、相邻区间、未知类型和零长度可接受；可用范围末端、保留区和空区间检查正确。
  这些验证没有修改真实 BIOS 缓冲或分配器。
- 分配器耗尽/恢复、IRQ/任务返回现场、SYSENTER 服务与边界请求、真实键盘 IRQ1、
  原 #DE/#PF 和四种用户保护仍通过，普通镜像没有主动异常。
- 最终 Loader 2,789 字节，普通 Kernel 86,504 字节，软盘 1,474,560 字节。
  `nm -u bin/system` 无未解析符号，ELF 入口保持 0xffff800000100000；
  普通用户载荷反汇编仍无段外调用或数据引用。Python 语法、Markdown 链接与 diff 空白检查通过。

新增关键日志：

```text
Loader E820 complete: version=1, stride=20, capacity=32
E820 handoff OK: version=1, entries=5, stride=20; complete, no overlaps
Boot memory layout OK: handoff, stack, VBE scratch, loader, temporary page tables
E820 input checks OK: header, overflow, overlaps, ordering, reserved ranges
```

最后一条只出现在 memory-check。证据保存在 `bin/smoke-{normal,feature}.log`，
`bin/boot-memory-map.bin` 是最后普通配置的完整 656 字节头/数组快照；生成物不提交。

限制：局部输入验证不能替代固件分支注入。未分别模拟 BIOS 的 20/24 字节、扩展属性、
CF 结束、错误长度/签名或超过 32 条返回；未注入 VBE 临时区超限。
拒绝重叠是一种保守限制，没有通用 E820 修复或任意碎片布局支持。
Rust 布局校验发生在 Loader/内核入口之后，不能追溯防止早期内存写入。
RFB/GUI、其他固件、NMI、U 盘/硬盘启动、UEFI 与真机的验证边界仍同前。
