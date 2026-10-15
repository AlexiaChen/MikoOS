/* SYSENTER 不压返回帧，RSP 已由 IA32_SYSENTER_ESP 加载，IF 已由 CPU 清零。
 * 不能先 STI：没有调度/嵌套协议时会重用服务栈，破坏尚未返回的现场。 */
.section .text,"ax"
.code64
.globl system_call_entry
.type system_call_entry,@function
system_call_entry:
    cld /* 用户可设置 DF；普通 Rust ABI 要求字符串操作方向向前。 */
    SAVE_REGISTERS
    movw $0x10, %ax
    movw %ax, %ds
    movw %ax, %es
    xorw %ax, %ax
    movw %ax, %fs
    movw %ax, %gs
    movq %rsp, %rdi
    movq %rsp, %rbx /* 原 RBX 已保存；C ABI 让 Rust 保留现场锚点。 */
    andq $-16, %rsp
    call system_call_dispatch
    movq %rbx, %rsp
    movq %rax, 112(%rsp) /* 只替换保存的 RAX，其余用户 GPR 全部恢复。 */
    movw $0x33, %ax
    movw %ax, %ds
    movw %ax, %es
    /* SYSEXIT 不恢复 RFLAGS，不能把服务内部计算的标志/权限残留给用户。
     * 本章有限演示只允许 bit1=1，其余包括 IF/IOPL/DF/TF/AC/NT 都清零。
     * POP 后直到 SYSEXIT 只有 MOV/POP，不再修改算术标志。 */
    pushq $2
    popfq
    RESTORE_REGISTERS
    .byte 0x48
    sysexit /* 已由 Rust 校验的 RDX→RIP、RCX→RSP，回到 CPL3。 */
.size system_call_entry,.-system_call_entry
