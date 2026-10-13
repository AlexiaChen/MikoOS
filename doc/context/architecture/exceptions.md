# 系统异常与 ABI

源码：`gate.rs`、`trap.rs`、`entry.S`，CPU 指令在 `arch.rs`。
GDT 由 head.S 加载；Rust 写入第 8~9 槽的 16 字节 TSS 描述符，初始化 TSS 后执行 `ltr 0x40`。
TSS 长度 104 字节，用未对齐写入遵守硬件偏移；I/O bitmap offset=104，当前无权限位图。

所有 IDT 条目大小 16 字节，CS=0x08，0~31 对应各 CPU 异常，其他条目进入通用 fatal 入口。
3/4/5 保留 DPL=3 的陷阱门；NMI 用中断门；其余用 DPL=0 陷阱门。
普通异常使用 IST1 的 32 KiB 独立栈；#DF 使用 IST2 的 16 KiB 独立栈。
不再把内核栈、RSP0、所有 IST 都设为 `0x7c00`。

entry.S 将错误码规范化：CPU 自带错误码的向量是 8、10~14、17、21、29、30，
其余手动压入 0。接着压入向量和 15 个通用寄存器；DF 清零后进入普通 Rust C ABI。
`ExceptionFrame` 的 RIP 偏移为 136（0x88），不是旧 C 入口的 0x98；编译期断言检查布局。
CALL 前重新 16 字节对齐 RSP，frame 指针保留对齐前的现场地址。
Rust 裸机目标无 red zone、不使用 SIMD/浮点寄存器，见
[目标说明](https://doc.rust-lang.org/rustc/platform-support/x86_64-unknown-none.html)。

异常打印 vector/error/RIP/旧 RSP；10~13 解码选择子索引、GDT/LDT/IDT、external；
14 打印 CR2 和 P/W/U/RSVD/I 位。修正旧 C 页异常错误使用 IDT_FLAG 解码 instruction-fetch 的问题。
当前所有异常处理后关中断停机，没有返回和 iretq，也没有外部 IRQ 驱动。
后续实现可恢复中断时，必须补全现场恢复，重新审视重入、IST 和 Rust ABI。

故障演示默认关闭。`fault-divide` 用真实 CPU div 触发 #DE，`fault-page` 读未映射地址触发 #PF；
不能同时开启，普通 Rust 除零只会触发 panic，不能替代 CPU 异常教学。
