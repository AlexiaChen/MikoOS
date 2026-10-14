/* 外部 IRQ 的 CPU 边界。硬件自动保存 RIP/CS/RFLAGS；IST3 切栈还保存旧 RSP/SS。
 * PIC IRQ 不带 CPU 错误码，补 0 和 vector 后复用异常现场布局。
 * PIC 的 EOI 由 Rust 发出，它只解除控制器的“正在服务”状态；恢复 CPU 现场仍
 * 必须靠 IRETQ。EOI 与 IRETQ 分别作用于两个不同的硬件状态机，不能互相替代。 */
.section .text,"ax"
.code64
.macro IRQ vector
.globl irq_\vector
irq_\vector:
    pushq $0
    pushq $\vector
    jmp irq_common
.endm
.irp vector,32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47
    IRQ \vector
.endr
irq_common:
    /* IF 已由中断门清零；不执行 STI，普通 IRQ 不会嵌套重用 IST3 栈顶。
     * 被打断代码可能设了 DF，而 Rust ABI 要求 DF=0；IRETQ 会恢复原 RFLAGS。 */
    cld
    SAVE_REGISTERS
    movq %rsp, %rdi
    /* RBX 在原值保存后可用作临时锚点，Rust C ABI 保证被调用函数保持 RBX。
     * 动态对齐可能留一个栈空隙，因此不能直接用 CALL 后的 RSP 恢复现场。 */
    movq %rsp, %rbx
    andq $-16, %rsp
    call irq_dispatch
    movq %rbx, %rsp
    RESTORE_REGISTERS
    addq $16, %rsp /* 丢弃软件压入的 vector 与占位 error，露出 CPU 返回帧。 */
    /* RET 只取回 RIP，而 IRETQ 还恢复 CS/RFLAGS/旧栈，尤其包括原来的 IF/DF。
     * 这使 IRQ 像在两条指令之间短暂插入一段代码，然后继续原来的执行流。 */
    iretq

.section .rodata,"a"
.balign 8
.globl irq_entries
irq_entries:
.irp vector,32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47
    .quad irq_\vector
.endr
