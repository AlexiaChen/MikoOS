# 外部中断：8259A、IRQ 与返回

对应父目录原版 `code/c04/4-9/kernel/{interrupt.c,entry.S,main.c}`。
Rust 入口与演示在 `src/kernel/interrupt.rs`，PIC 协议在 `interrupt/pic.rs`，
CPU 保存/返回边界在 `irq.S`，TSS/IDT 分别由 `gate.rs`、`trap.rs` 管理。

## 核心调用链

```text
Start_Kernel：所有内存初始化与分配检查结束，IF 仍为 0
  → interrupt::init：安装 16 个门 → 重设两片 PIC → 所有线屏蔽
  → 可选 irq-check：软件 INT 验证现场/伪中断分支，不冒充外部中断
  → demonstrate_timer：设置 PIT channel0 → 仅开放 IRQ0 → STI/HLT/CLI 等待
      → PIT 请求 → 主 PIC ISR0 置位 → 向量 0x20 → CPU 中断门清 IF 并切到 IST3
      → irq_common 保存全部 GPR → irq_dispatch 核对 ISR0、计数、发送 EOI
      → 恢复 GPR → IRETQ 恢复原 RIP/CS/RFLAGS/RSP/SS → 从 HLT 后继续
  → 收到两次后全屏蔽、IF=0 → keyboard::init 只开放 IRQ1 → ready
  → 可选故障演示（kernel_halt）或 keyboard::run 持续等待并消费输入
```

处理函数不打印普通 IRQ 日志，主流程关中断后报告结果，避免打断控制台输出。
PIT 输入约 1,193,182 Hz，除数 11,932、mode2，约 100 Hz；仅用于稳定交付两个事件，
不提供时间 API、睡眠、调度或周期后台任务。演示结束时 PIT 仍计数，但 IRQ0 已屏蔽；
之后键盘初始化单独开放 IRQ1，不重新开放时钟。

## PIC 与 CPU 各自的状态

PIC 的 IRR 保存待处理请求，IMR 的 1 位屏蔽该线，ISR 保存正在服务的请求；
CPU 的 IF 只控制是否接受可屏蔽中断。EOI 解除 PIC 的在服务状态，IRETQ 恢复 CPU 的执行现场，
两者不能互相代替。CLI、屏蔽线都不负责清除设备本身的状态。

主/从片命令与数据端口是 0x20/0x21、0xa0/0xa1。ICW1=0x11，ICW2=0x20/0x28，
ICW3=0x04/0x02（主 IRQ2 级联），ICW4=0x01（8086、非 Auto-EOI）。
ICW 写后用传统 POST 端口 0x80 延迟。初始化后两片 IMR=0xff；时钟演示主片改为 0xfe，
时钟结束后重新全屏蔽，键盘就绪再改主片为 0xfd，仅开放 IRQ1。
PIC 16 路映射到 32~47，避开 CPU 的 0~31 异常；不照搬原版 24 个入口或全开放的 IMR。

OCW3=0x0b 选择 ISR 读回。IRQ0 必须确认 ISR0=1 才计数，每次 specific EOI 后核对 ISR0=0；
第二次交付证明第一轮服务已结束且 CPU 已返回。真实从片 IRQ 的 EOI 先给从片输入，再给主片 IRQ2。
IRQ1 同样核对 ISR1，每次先由 keyboard::on_interrupt 消费 8042 输出，再 EOI 并核对 ISR1 清零；
不得对其他 IRQ 随意读 0x60。设备状态消费与 PIC EOI 的区别见 [键盘](keyboard.md)。
IRQ7/15 若相应 ISR 位为 0，则为伪中断：IRQ7 不发 EOI，IRQ15 只给主 IRQ2 EOI。
软件验证触发这两个 ISR 为空的分支，不能证明实际电气条件下发生过伪中断或真实从片交付。

## 栈、现场与 Rust ABI

IRQ 门是 0x8e（present、DPL0、中断门）、CS=0x08、IST=3。中断门由 CPU 清 IF，
处理函数不重新开放，不会递归切到同一 IST3 栈顶；异常和 #DF 分别用 IST1/IST2。
新增 IRQ 栈 32 KiB，属于链接器 BSS，在建立内存元数据前已被包含进内核预留区域。
不复用原版低端 0x7c00 栈，也不让 IRQ 与异常共用栈顶。

CPU 帧与软件 vector/error 加上 15 个 GPR，形成 trap::InterruptFrame，共 176 字节。
entry.S 提供 SAVE/RESTORE 宏，irq.S 在调用 Rust 前 CLD，并按 System V ABI 对齐栈到 16 字节。
保存后的 RBX 作原现场地址锚点，Rust callee-saved 约定保持它；CALL 返回后恢复原 RSP 再出栈。
IRETQ 恢复 CPU 帧，包括被打断时的 IF/DF；裸机目标无 red zone，不使用 SIMD/浮点，
尚未保存 SIMD/FPU 状态。新增这些能力时必须先扩展入口协议。

`arch::wait_for_interrupt` 把 STI/HLT/CLI 放在一个汇编块：STI 的中断延迟覆盖紧随的 HLT，
避免事件先处理完才睡下的竞态，返回时 CLI。初始化及内存分配阶段均 IF=0；
键盘主流程在 IF=0 下消费输入和打印，队列为空后才开放等待窗口。
没有锁、SMP、可嵌套 IRQ 或中断中分配内存支持；未知真实设备 IRQ 直接诊断停机。

## 专用验证

`irq-check` 在全屏蔽、ISR 为空时运行 `irq_check.S`：全部 15 个 GPR 放哨兵，
设置 DF/算术标志且 IF=0，通过软件 INT 0x27 检查 GPR、RSP、CF/PF/AF/ZF/SF/IF/DF/OF 恢复；
再用软件 INT 0x2f 检查另一个伪中断分支，要求计数各一次且 PIT 计数仍为零。
它只验证 CPU 入口/返回和处理分支；真实 PIC 交付由之后的两次 PIT IRQ0 单独证明。
命令与实际结果见 [构建](../development/build.md) 和 [验证记录](../development/validation.md)。
