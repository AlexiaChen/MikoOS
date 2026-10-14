/* 主动任务切换的 CPU 边界。调用者通过普通 C ABI 请求切换，因此编译器已经
 * 为调用后仍需要的 caller-saved 值作了安排；这里只须保存六个 callee-saved GPR。
 * 异步 IRQ 无这个调用约定前提，必须保存全部 GPR，见 entry.S。
 * 本节约定所有切换都在 IF=0、DF=0 下进行，状态标志不是普通函数的返回契约。 */
.section .text,"ax"
.code64
.globl task_context_switch
task_context_switch:
    pushq %rbp
    pushq %rbx
    pushq %r12
    pushq %r13
    pushq %r14
    pushq %r15
    /* RDI=旧 Context 指针，RSI=新 Context 指针；Context 的首字段是 saved RSP。
     * CALL 已将继续执行的 RIP 压在旧栈上，六个寄存器保存在它下面。
     * 保存这个 RSP 就同时找到了“寄存器快照”和“稍后从哪里继续”的返回地址。 */
    movq %rsp, (%rdi)
    movq (%rsi), %rsp
    popq %r15
    popq %r14
    popq %r13
    popq %r12
    popq %rbx
    popq %rbp
    /* 从新栈取 RIP。第一次指向构造的 trampoline，之后指向该任务上一次 CALL
     * 的下一条指令。这次 RET 可以回到另一条早先暂停的调用链，而非当前调用者。 */
    ret

/* trampoline 按设计永不返回。这个保护性返回地址接住未来误改造成的 RET：
 * RET 已额外弹走 8 字节，先补回一个槽位，再按 C ABI 跳入 Rust 的诊断函数。 */
.globl task_returned
task_returned:
    subq $8, %rsp
    jmp task_unexpected_return
