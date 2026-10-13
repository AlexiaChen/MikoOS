/* 64 位启动边界。Loader 已开启 long mode，并将 kernel.bin 放到物理 1 MiB。
 * .org 固定三级页表的位置，CR3 的物理地址必须与此保持一致。
 * 普通内核功能在 Rust 中；这里只建立 Rust 运行所需的机器状态。 */
.section .text.boot,"ax"
.code64
.globl _start
_start:
    cli
    cld
    /* 根据选定 VBE 模式交接的物理帧缓冲设置 PDE，避免假定显存固定在 E0000000。 */
    movl 0x8228, %eax
    andl $0xffe00000, %eax
    orq $0x83, %rax
    leaq __PDE+40(%rip), %rdi
    movl $8, %ecx
1:
    movq %rax, (%rdi)
    addq $0x200000, %rax
    addq $8, %rdi
    loop 1b
    movq $0x101000, %rax
    movq %rax, %cr3
    lgdt GDT_POINTER(%rip)
    /* far return 同时重新加载 CS，并跳转到链接器定义的高半区地址。 */
    movabsq $entry64, %rax
    pushq $0x08
    pushq %rax
    lretq
entry64:
    movw $0x10, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %ss
    movw %ax, %fs
    movw %ax, %gs
    /* objcopy 的平坦二进制不包含 NOBITS；必须主动清零 BSS，不能依赖 BIOS。 */
    leaq _bss(%rip), %rdi
    leaq _endbss(%rip), %rcx
    subq %rdi, %rcx
    xorl %eax, %eax
    rep stosb
    /* 使用内核专属 64 KiB 栈，不再与引导扇区/E820/TSS 共用 0x7c00。 */
    leaq boot_stack_top(%rip), %rsp
    andq $-16, %rsp
    xorq %rbp, %rbp
    /* System V ABI: CALL 前 RSP 是 16 字节对齐，函数入口 RSP % 16 == 8。 */
    call Start_Kernel
2:  cli
    hlt
    jmp 2b

.org 0x1000
__PML4E:
    .quad 0x102003
    .fill 255,8,0
    .quad 0x102003
    .fill 255,8,0
.org 0x2000
__PDPTE:
    .quad 0x103003
    .fill 511,8,0
.org 0x3000
__PDE:
    .quad 0x000083,0x200083,0x400083,0x600083,0x800083
    .fill 8,8,0 /* 显存 PDE 在 _start 中根据 VBE 填写 */
    .fill 499,8,0

.section .data.boot,"aw"
.balign 16
.globl GDT_Table
GDT_Table:
    .quad 0,0x0020980000000000,0x0000920000000000
    .quad 0x0020f80000000000,0x0000f20000000000
    .quad 0x00cf9a000000ffff,0x00cf92000000ffff,0
    .quad 0,0 /* GDT 8~9: Rust 设置 16 字节 TSS 描述符 */
GDT_END:
GDT_POINTER:
    .word GDT_END-GDT_Table-1
    .quad GDT_Table

.section .bss.stack,"aw",@nobits
.balign 16
boot_stack:
    .skip 65536
boot_stack_top:
