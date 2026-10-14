/* 实现完成后的主动切换验证：让六个 callee-saved GPR 和本栈局部哨兵跨越
 * 一次真实的暂停/恢复。外层 Rust 调用者的寄存器另存，遵守本函数自身的 C ABI。 */
.section .text,"ax"
.code64
.globl task_check_switch
task_check_switch:
    pushq %rbp
    pushq %rbx
    pushq %r12
    pushq %r13
    pushq %r14
    pushq %r15
    /* 函数入口 RSP%16=8；6 次 PUSH 后仍为 8，再减 72 使 CALL 前对齐为 0。
     * 以旧 Context 地址为种子，使两任务的哨兵不同。若所有任务放相同常量，
     * 错误地沿用另一任务的寄存器也可能碰巧通过检查。期望值保存在自己的栈中。 */
    subq $72, %rsp
    movq %rsp, (%rsp)
    movq $0x12345678, 8(%rsp)
.macro TASK_SENTINEL register offset salt
    movq %rdi, %rax
    xorq $\salt, %rax
    movq %rax, \offset(%rsp)
    movq %rax, %\register
.endm
    TASK_SENTINEL rbp,16,0x1111
    TASK_SENTINEL rbx,24,0x2222
    TASK_SENTINEL r12,32,0x3333
    TASK_SENTINEL r13,40,0x4444
    TASK_SENTINEL r14,48,0x5555
    TASK_SENTINEL r15,56,0x6666
    call task_context_switch
    /* 恢复时才会从这里继续；这些值不能由另一个任务的寄存器现场代替。 */
    cmpq %rbp, 16(%rsp)
    jne .Ltask_bad
    cmpq %rbx, 24(%rsp)
    jne .Ltask_bad
    cmpq %r12, 32(%rsp)
    jne .Ltask_bad
    cmpq %r13, 40(%rsp)
    jne .Ltask_bad
    cmpq %r14, 48(%rsp)
    jne .Ltask_bad
    cmpq %r15, 56(%rsp)
    jne .Ltask_bad
    cmpq %rsp, (%rsp)
    jne .Ltask_bad
    cmpq $0x12345678, 8(%rsp)
    jne .Ltask_bad
    xorl %eax, %eax
    jmp .Ltask_checked
.Ltask_bad:
    movl $1, %eax
.Ltask_checked:
    addq $72, %rsp
    popq %r15
    popq %r14
    popq %r13
    popq %r12
    popq %rbx
    popq %rbp
    ret
