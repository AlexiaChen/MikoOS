/* 第 5 章 5-1：SYSEXIT 的硬件边界。策略、页表和检查都由 Rust 负责。
 * user_enter 是普通 C ABI 函数；虽然它中途去 ring3，最终仍须像普通函数一样
 * 把调用者的 callee-saved 寄存器和栈恢复。用户程序不能决定这个返回地址。 */
.section .text,"ax"
.code64
.globl user_enter
.type user_enter,@function
user_enter:
    pushq %rbp
    pushq %rbx
    pushq %r12
    pushq %r13
    pushq %r14
    pushq %r15
    movq %rsp, user_kernel_rsp(%rip)
    /* Intel SYSEXIT64：RDX→RIP，RCX→RSP；它不从栈弹出 IRETQ 现场。
     * IA32_SYSENTER_CS=8，因此 CS=(8+32)|3=0x2b，SS=(8+40)|3=0x33。
     * 段的隐藏缓存由 CPU 构造，不通过 GDT 读取；DS/ES 则仍需软件准备。 */
    movq %rdi, %rdx
    movq %rsi, %rcx
    movw $0x33, %ax
    movw %ax, %ds
    movw %ax, %es
    xorl %eax, %eax
    movw %ax, %fs
    movw %ax, %gs
    /* 不把内核栈地址等残留寄存器内容带给用户。RCX/RDX 是约定的用户地址。
     * SYSEXIT 不恢复 RFLAGS；明确使用 bit1=1、IF=IOPL=DF=0，不能把内核的
     * I/O 权限或方向标志遗留给用户。本节是有限演示，不让 IRQ 打断用户代码。 */
    xorq %rax, %rax
    xorq %rbx, %rbx
    xorq %rbp, %rbp
    xorq %rdi, %rdi
    xorq %rsi, %rsi
    xorq %r8, %r8
    xorq %r9, %r9
    xorq %r10, %r10
    xorq %r11, %r11
    xorq %r12, %r12
    xorq %r13, %r13
    xorq %r14, %r14
    xorq %r15, %r15
    pushq $2
    popfq
    /* REX.W 是关键：不加它，SYSEXIT 会进入 32 位兼容模式。 */
    .byte 0x48
    sysexit
.size user_enter,.-user_enter

.globl user_return_entry
.type user_return_entry,@function
user_return_entry:
    /* DPL3 中断门已使 CPL=0、IF=0，且由 TSS.RSP0 提供独立内核栈。
     * 用户栈及其返回地址都不可信，不能在上面运行 Rust，也不能用它恢复内核。 */
    cld
    pushq $0
    pushq $0x80
    SAVE_REGISTERS
    movw $0x10, %ax
    movw %ax, %ss
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %fs
    movw %ax, %gs
    movq %rsp, %rdi
    andq $-16, %rsp
    call user_return_dispatch
    /* 完成通知终止本次用户演示，返回的是内核事先保存的 C ABI 调用现场。
     * 它不支持任意系统调用，也不恢复用户执行；下一节再讨论服务入口协议。 */
    movq user_kernel_rsp(%rip), %rsp
    popq %r15
    popq %r14
    popq %r13
    popq %r12
    popq %rbx
    popq %rbp
    retq
.size user_return_entry,.-user_return_entry

.section .bss,"aw",@nobits
.balign 8
user_kernel_rsp:
    .skip 8
