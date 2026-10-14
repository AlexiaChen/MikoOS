# 首个内核线程与主动切换

参考父目录原版 `code/c04/4-11/kernel/{task.c,task.h,main.c}`，对应 kernel_thread/init/switch_to。
源码 `src/kernel/task.rs`，CPU 换栈边界 `task_switch.S`；扩展验证 `task_check.S`。
本节只有启动任务和一个 init 内核线程，共享内核页表、均为 ring0、切换时 IF=0。
没有调度队列、时间片、抢占、用户进程、一般 fork 或文件/信号等共享资源。

## 主线与状态

```text
内存/PIC/两次 IRQ0 演示完成，PIC 全屏蔽且 IF=0
  → task::demonstrate 将已有启动栈登记为 pid0
  → Mapped 分配一页，显式构造 pid1/Ready/init(10) 与首次切入帧
  → 启动任务 Ready → init Running：在独立栈上开始函数调用链
  → init 记录 progress=1，主动切回：init Ready → 启动任务 Running
  → 启动任务检查让出状态，再次切入 init
  → init 从上一次 switch_to 调用后继续，progress=2，函数返回退出码 1
  → trampoline 记录 Zombie/退出码，切回启动任务，不再返回
  → 启动任务检查栈哨兵、CPU 栈/TSS/CR3，归还整页并验证再分配复用
  → 初始化键盘 → ready → 持续键盘交互
```

Task 保存 Context、pid、Ready/Running/Zombie、栈边界、共享 CR3、函数/参数、退出码及演示进度。
NEXT_PID 单调递增并检查溢出；当前只创建一个子线程。没有通用任务链表或僵尸队列。
KernelThread 的 PageRun 凭据一直由启动任务持有，子线程函数返回后先结束自身局部资源，
再转 Zombie。退出与回收是两步：正在使用的栈和返回地址不能被当前执行流归还给分配器。

## 独立栈与首次进入

Mapped 页完整直映，大小仍为 2 MiB；本节沿原版的 32 KiB 任务区域思路，
只使用页前 32 KiB，其余保持归整页所有者，未提供子页分配。
默认页虚拟基址 V=`0xffff800000200000`：

| 相对 V 的范围 | 用途 |
| --- | --- |
| 0..4096 | Task 描述符，编译期限制大小不超过 4 KiB |
| 4096..4104 | 8 字节栈边界哨兵 |
| 4104..32768 | 向低地址增长的线程栈，可用 28 KiB 减 8 字节 |
| 32768..2 MiB | 当前未用，仍占用整页，不得重复分配 |

初始 Context.rsp=top-64；8 个 u64 从低到高是 r15/r14/r13/r12/rbx/rbp、trampoline RIP、
保护性返回地址。切入时弹出 6 个寄存器再 RET，trampoline 入口 RSP=top-8，满足 C ABI 的
RSP%16=8。保护返回入口先补回 8 字节对齐槽再跳入 Rust panic；正常 trampoline 永不返回。
这是假造首次函数入口的调用现场，不是异常返回帧，不使用 IRETQ。

哨兵只能发现覆盖到该字的错误，不能代替未映射 guard page。没有栈扩容或可靠溢出防护。
head.S 导出 boot_stack/boot_stack_top，使启动任务的已有 64 KiB 栈无需移动或复制。

## 为什么保存六个寄存器就能主动切换

外部 C ABI 的 task_context_switch(previous_context, next_context) 有普通函数调用的约定。
编译器负责 caller-saved 值；汇编保存 RBP/RBX/R12..R15，把 RSP 写到旧 Context，再载入新 RSP。
CALL 早已把继续执行的 RIP 放在旧栈上，故 Context 只需一个 rsp 字段（repr(C)、偏移 0、大小 8）。
恢复新栈上的六个寄存器并 RET，就能继续该任务上次暂停的调用链。
RFLAGS 的算术位属于函数调用的易失状态；本节要求所有切换 IF=0、DF=0，不保存独立的 IF。
异步 IRQ 无调用者预先保存现场的前提，仍保存全部 GPR，并通过 IRETQ 恢复 RFLAGS，不能混用协议。

独立汇编函数让编译器明确看到完整调用边界；不在普通 Rust 内联汇编中悄悄改 RSP。
Rust 使用短期 raw pointer 读写描述符，不持有 &mut Task 跨暂停；另一个执行流会修改同一任务的状态。
只有共享 CR3 被核对，不重载页表；FS/GS/基址在本节不改变，无 TLS/SIMD/FPU 上下文。
将来引入用户态、不同地址空间、SMP、抢占或可变段/TLS 状态时需扩展协议。

## 当前任务与 TSS

CURRENT 显式记录任务指针，不用原版 RSP & ~32767 推断地址。
原版 task_union 声明只要求 8 字节对齐，但 Kernel.lds 会先 ALIGN(32768) 再放置 .data.init_task，
所以启动任务依靠链接布局满足掩码要求；子线程的 2 MiB 页也满足对齐。
本项目的启动栈、IST 栈不处于同一种布局，不能把 IRQ 栈地址当任务描述符，因此显式记录 CURRENT。

切换前在 IF=0 下更新状态、CURRENT 和 TSS.RSP0，再执行换栈。此小段过渡不支持 NMI/SMP 读取任务状态。
gate::set_kernel_stack 只用未对齐写更新 TSS 偏移 4，IST1/2/3 不动；TR 仍指向同一个 TSS。
RSP0 为将来从较低特权级进入内核准备栈，ring0 主动切换本身不会使用它自动换栈。
每次恢复检查当前实际 RSP 范围、TSS 读回、CR3 与 Running 状态；这不证明已经验证过 ring3 进入。

原版 do_fork 整体复制后再修订描述符/现场，本项目改为显式构造有效字段与新帧，避免旧指针别名依赖。
原版 lib.h 自定义 memcpy(From, To, Num)，其 regs→新栈调用符合该接口；本项目 arch.rs 的
裸机 memcpy 遵循标准 C 的 dst/src 顺序，不能直接移植调用参数并误判原版方向错误。
原版 do_exit 无限停留改为 Zombie 后切回启动任务并异栈回收。未伪装实现 CLONE_FS/FILES/SIGNAL 等接口。

## 专用验证

task-check 将真实 switch 包在汇编检查入口中：以旧 Context 地址为种子，为两个任务产生不同的
六个 callee-saved 哨兵，把期望值、原 RSP 与栈数据存在各自栈中。恢复后逐一核对；
共有三次完成返回（启动任务两次、init 主动让出后一次），退出线程的最后一条调用不再返回。
init 上另走软件 INT 0x27 的 IST3/IRETQ，确认回到原任务栈，不能当真实设备 IRQ 或抢占验证。
所有配置的正常演示也检查函数参数/局部值、进度/退出码、状态转换、边界哨兵和回收后原页可复用。
命令见 [构建](../development/build.md)，实际运行证据见 [验证](../development/validation.md)。
