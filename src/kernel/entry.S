/* CPU 异常入口：统一有/无硬件错误码的栈形状，再调用普通 Rust C ABI。
 * 使用独立 IST 栈；本章异常打印后停机，故不实现恢复现场/iretq。
 * 若后续实现可恢复中断，必须同时保存/恢复全部现场并补上 iretq。 */
.section .text,"ax"
.code64
.macro EXCEPTION vector has_error=0
.globl exception_\vector
exception_\vector:
.if !\has_error
    pushq $0
.endif
    pushq $\vector
    jmp exception_common
.endm
EXCEPTION 0
EXCEPTION 1
EXCEPTION 2
EXCEPTION 3
EXCEPTION 4
EXCEPTION 5
EXCEPTION 6
EXCEPTION 7
EXCEPTION 8,1
EXCEPTION 9
EXCEPTION 10,1
EXCEPTION 11,1
EXCEPTION 12,1
EXCEPTION 13,1
EXCEPTION 14,1
EXCEPTION 15
EXCEPTION 16
EXCEPTION 17,1
EXCEPTION 18
EXCEPTION 19
EXCEPTION 20
EXCEPTION 21,1
EXCEPTION 22
EXCEPTION 23
EXCEPTION 24
EXCEPTION 25
EXCEPTION 26
EXCEPTION 27
EXCEPTION 28
EXCEPTION 29,1
EXCEPTION 30,1
EXCEPTION 31
EXCEPTION 255
exception_common:
    cli
    cld
    /* 顺序与 trap.rs::ExceptionFrame 一致：最后压入的 r15 位于偏移 0。 */
    pushq %rax
    pushq %rbx
    pushq %rcx
    pushq %rdx
    pushq %rbp
    pushq %rdi
    pushq %rsi
    pushq %r8
    pushq %r9
    pushq %r10
    pushq %r11
    pushq %r12
    pushq %r13
    pushq %r14
    pushq %r15
    movq %rsp, %rdi
    andq $-16, %rsp
    call exception_dispatch
1:  hlt
    jmp 1b
.section .rodata,"a"
.balign 8
.globl exception_entries
exception_entries:
.macro POINTER vector
    .quad exception_\vector
.endm
.irp n,0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31
    POINTER \n
.endr
