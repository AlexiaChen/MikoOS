/* 实现后的协议验证：这段代码也复制到用户代码页，真实从 CPL3 执行 SYSENTER。
 * 普通 C ABI 调用允许破坏易失 GPR，故需在汇编里为服务协议的全部 GPR 放哨兵。 */
.section .user.check,"ax"
.code64
.globl user_syscall_check
.type user_syscall_check,@function
user_syscall_check:
    pushq %rbx
    pushq %rbp
    pushq %r12
    pushq %r13
    pushq %r14
    pushq %r15
    subq $16, %rsp
    movq %rsp, (%rsp)
    movq $0x1234, 8(%rsp)
    movq $15, %rax
    movq $0x2222, %rbx
    movq $0x5555, %rbp
    movq $0x6666, %rdi
    movq $0x7777, %rsi
    movq $0x8888, %r8
    movq $0x9999, %r9
    movq $0xaaaa, %r10
    movq $0xbbbb, %r11
    movq $0xcccc, %r12
    movq $0xdddd, %r13
    movq $0xeeee, %r14
    movq $0xffff, %r15
    leaq .Lsyscall_user_return(%rip), %rdx
    movq %rsp, %rcx
    /* DF=1 只存在于纯汇编；内核须先 CLD。SYSENTER 不保存 RFLAGS，
     * SYSEXIT 返回时按本项目协议清算术标志/DF，不能照搬 IRQ 的标志恢复规则。 */
    pushq $0x447
    popfq
    sysenter
.Lsyscall_user_return:
    pushfq /* 必须在 CMP 修改算术标志之前取样。 */
.macro CHECK_SYSCALL_REGISTER register value
    cmpq $\value, %\register
    jne .Lsyscall_bad_register
.endm
    CHECK_SYSCALL_REGISTER rax,-38
    CHECK_SYSCALL_REGISTER rbx,0x2222
    CHECK_SYSCALL_REGISTER rbp,0x5555
    CHECK_SYSCALL_REGISTER rdi,0x6666
    CHECK_SYSCALL_REGISTER rsi,0x7777
    CHECK_SYSCALL_REGISTER r8,0x8888
    CHECK_SYSCALL_REGISTER r9,0x9999
    CHECK_SYSCALL_REGISTER r10,0xaaaa
    CHECK_SYSCALL_REGISTER r11,0xbbbb
    CHECK_SYSCALL_REGISTER r12,0xcccc
    CHECK_SYSCALL_REGISTER r13,0xdddd
    CHECK_SYSCALL_REGISTER r14,0xeeee
    CHECK_SYSCALL_REGISTER r15,0xffff
    cmpq %rcx, 8(%rsp) /* 保存的用户 RSP。 */
    jne .Lsyscall_bad_register
    leaq .Lsyscall_user_return(%rip), %rax
    cmpq %rax, %rdx
    jne .Lsyscall_bad_register
    popq %rax
    cmpq $2, %rax
    jne .Lsyscall_bad_frame
    cmpq %rsp, (%rsp)
    jne .Lsyscall_bad_frame
    cmpq $0x1234, 8(%rsp)
    jne .Lsyscall_bad_frame
    xorl %eax, %eax
    jmp .Lsyscall_checked
.Lsyscall_bad_register:
    addq $8, %rsp
.Lsyscall_bad_frame:
    movl $1, %eax
.Lsyscall_checked:
    cld
    addq $16, %rsp
    popq %r15
    popq %r14
    popq %r13
    popq %r12
    popq %rbp
    popq %rbx
    retq
.size user_syscall_check,.-user_syscall_check
