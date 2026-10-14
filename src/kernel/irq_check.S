/* 实现完成后的现场恢复验证。普通 C ABI 调用者已在函数调用前同意丢弃易失
 * 寄存器，无法检测 IRQ 是否破坏它们，所以在汇编里给全部 GPR 放入哨兵。
 * 软件 INT 只能验证 CPU 入口/返回，不能冒充 PIC 设备中断；硬件交付另用 PIT 验证。 */
.section .text,"ax"
.code64
.globl irq_verify_context
irq_verify_context:
    /* 本函数仍是普通 Rust extern C 的被调用者，须保留外层 ABI 的 callee-saved。 */
    pushq %rbx
    pushq %rbp
    pushq %r12
    pushq %r13
    pushq %r14
    pushq %r15
    subq $16, %rsp
    movq %rsp, (%rsp)
    pushfq
    popq %rax
    movq %rax, 8(%rsp)
    /* IF=0，DF=1，CF/PF/ZF=1；中断入口须 CLD 供 Rust 使用，返回须恢复 DF。
     * 只在这段纯汇编里暂时设置 DF，不允许以 DF=1 调用普通 Rust 函数。 */
    pushq $0x447
    popfq
    movq $0x1111, %rax
    movq $0x2222, %rbx
    movq $0x3333, %rcx
    movq $0x4444, %rdx
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
    int $0x27
    /* 在 CMP 改写算术标志前保存返回时的 RFLAGS，不占用任何哨兵寄存器。 */
    pushfq
.macro CHECK_REGISTER register value
    cmpq $\value, %\register
    jne .Lirq_bad_register
.endm
    CHECK_REGISTER rax,0x1111
    CHECK_REGISTER rbx,0x2222
    CHECK_REGISTER rcx,0x3333
    CHECK_REGISTER rdx,0x4444
    CHECK_REGISTER rbp,0x5555
    CHECK_REGISTER rdi,0x6666
    CHECK_REGISTER rsi,0x7777
    CHECK_REGISTER r8,0x8888
    CHECK_REGISTER r9,0x9999
    CHECK_REGISTER r10,0xaaaa
    CHECK_REGISTER r11,0xbbbb
    CHECK_REGISTER r12,0xcccc
    CHECK_REGISTER r13,0xdddd
    CHECK_REGISTER r14,0xeeee
    CHECK_REGISTER r15,0xffff
    popq %rax
    /* 核对 CF/PF/AF/ZF/SF/IF/DF/OF；不要求保留无关保留位的读回形式。 */
    andq $0xed5, %rax
    cmpq $0x445, %rax
    jne .Lirq_bad_frame
    cmpq %rsp, (%rsp)
    jne .Lirq_bad_frame
    xorl %eax, %eax
    jmp .Lirq_checked
.Lirq_bad_register:
    addq $8, %rsp
.Lirq_bad_frame:
    movl $1, %eax
.Lirq_checked:
    pushq 8(%rsp)
    popfq
    cld
    addq $16, %rsp
    popq %r15
    popq %r14
    popq %r13
    popq %r12
    popq %rbp
    popq %rbx
    ret
