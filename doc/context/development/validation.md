# 验证记录

最新状态见末尾的 **2026-10-15：5-1 用户态入口**；以下保留各阶段当时的验证边界。

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
