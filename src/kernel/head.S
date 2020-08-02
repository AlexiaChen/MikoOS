/* 
Kernel execution head to create segment,page table structure and set some of registers
Layout of physical memory
begin plat address 0xffff00000000000 physical address 0
begin address of kernel 0xffff00000000000 ~ 0x100000 (1MB)
|-------------BootLoader(0 - 1BM)--------|---head.S---|-------------------------Kernel---------
*/


.section .text

.globl _start

//====== default begin flag in GAS compiler
_start:

	mov	$0x10,	%ax
	mov	%ax,	%ds
	mov	%ax,	%es
	mov	%ax,	%fs
	mov	%ax,	%ss
	mov	$0x7E00,	%esp

//=======	load GDTR using RIP-Relative mode in IA-32e

	lgdt	GDT_POINTER(%rip)

//=======	load	IDTR

	lidt	IDT_POINTER(%rip)

	mov	$0x10,	%ax
	mov	%ax,	%ds
	mov	%ax,	%es
	mov	%ax,	%fs
	mov	%ax,	%gs
	mov	%ax,	%ss

	movq	$0x7E00,	%rsp

//=======	load	cr3

	movq	$0x101000,	%rax
	movq	%rax,		%cr3
	movq	switch_seg(%rip),	%rax
	pushq	$0x08
	pushq	%rax
	lretq

//=======	64-bit mode code

switch_seg:
	.quad	entry64

entry64:
	movq	$0x10,	%rax
	movq	%rax,	%ds
	movq	%rax,	%es
	movq	%rax,	%gs
	movq	%rax,	%ss
	movq	$0xffff800000007E00,	%rsp		/* rsp address */

/*

// Exceptions are divided into three types: fault, trap and abort

// fault: An fault is an exception that can be fixed. 
// As long as the error is corrected, the processor can restore the running environment of the program or task to before the exception occurs (CS and EIP registers have been saved in the stack), 
// and re-run the exception instruction, that is to say, the return address of the exception points to the instruction generating the fault, rather than the position after it

// trap: The trap also allows the processor to continue instructing the program or task, but the processor will skip the instruction generating the exception, 
// that is, the return address of the trap exception points to the address after the trap instruction is induced

// abort: The abort exception user reports a very serious error, which often fails to provide the exact location of the exception, and does not allow the program or task to continue to execute. 
// Typical termination exceptions have hardware errors

// Intel® 64 and IA-32 ArchitecturesSoftware Developer’s Manual Volume 3 
// https://www.intel.com/content/www/us/en/architecture-and-technology/64-ia-32-architectures-software-developer-system-programming-manual-325384.html


// In fact, interrupt and exception vectors are in the same IDT table, but their vector numbers are different. 
// The first 32 vector numbers of the IDT table are occupied by exceptions, 
// and the vector numbers of each exception cannot be modified. They are occupied by the interrupt handler starting from the vector number 32

*/

setup_IDT:							
	/*
	// Initialize each interrupt descriptor in the IDT (256 entries in total, 16 bytes each). 
	// It will ignore_ The starting address and other configuration information of int module are formatted into the interrupt gate descriptor structure information of IA-32e mode orderly, 
	// and the structure information is saved in rax register (the lower 8 bytes of structure information) and RDX register (the higher 8 bytes of structure information).
	*/
	leaq	ignore_int(%rip),	%rdx
	movq	$(0x08 << 16),	%rax
	movw	%dx,	%ax
	movq	$(0x8E00 << 32),	%rcx		
	addq	%rcx,	%rax
	movl	%edx,	%ecx
	shrl	$16,	%ecx
	shlq	$48,	%rcx
	addq	%rcx,	%rax
	shrq	$32,	%rdx
	leaq	IDT_Table(%rip),	%rdi
	mov	$256,	%rcx
rp_sidt:
	/* Finally, initializes 256 interrupt descriptors */
	movq	%rax,	(%rdi)
	movq	%rdx,	8(%rdi)
	addq	$0x10,	%rdi
	dec	%rcx
	jne	rp_sidt

setup_TSS64:
	/* 
	After the completion of IDT initialization, the task state segment descriptor (TSS descriptor) needs to be initialized. 
	// This part of the program initializes TSS descriptor in GDT (IA-32e mode), and loads the segement selector  of TSS descriptor into TR register through LTR assembly instruction. 
	// Because the current kernel program has been running under ring 0, even if an exception is generated, the task stack will not be switched, 
	// so there is no need to access TSS, so TSS does not need to be initialized temporarily. 
	// That is to say, in the case of no privilege level change, even if the selector of TSS descriptor is not loaded to TR register
	*/
	leaq	TSS64_Table(%rip),	%rdx
	xorq	%rax,	%rax
	xorq	%rcx,	%rcx
	movq	$0x89,	%rax
	shlq	$40,	%rax
	movl	%edx,	%ecx
	shrl	$24,	%ecx
	shlq	$56,	%rcx
	addq	%rcx,	%rax
	xorq	%rcx,	%rcx
	movl	%edx,	%ecx
	andl	$0xffffff,	%ecx
	shlq	$16,	%rcx
	addq	%rcx,	%rax
	addq	$103,	%rax
	leaq	GDT_Table(%rip),	%rdi
	movq	%rax,	64(%rdi)
	shrq	$32,	%rdx
	movq	%rdx,	72(%rdi)

	/*mov	$0x40,	%ax
	ltr	%ax*/

	movq	go_to_kernel(%rip),	%rax		/* movq address */
	pushq	$0x08
	pushq	%rax
	lretq

go_to_kernel:
	.quad	Start_Kernel


//=======	ignore_int

ignore_int:
	/*
	// this program first saves the values of each register, and then sets the DS and ES segment registers to kernel data segments, 
	// followed by color_printk function prepares the parameters and pass them by register.
	// After the prompt information is displayed, execute the JMP instruction loop

	// pass parameter from x64 mode, pass from left to right is RDI,RSI,RDX,RCX,R8,R9, rest of parameters passed by memory
	*/
	cld
	pushq	%rax
	pushq	%rbx
	pushq	%rcx
	pushq	%rdx
	pushq	%rbp
	pushq	%rdi
	pushq	%rsi

	pushq	%r8
	pushq	%r9
	pushq	%r10
	pushq	%r11
	pushq	%r12
	pushq	%r13
	pushq	%r14
	pushq	%r15

	movq	%es,	%rax
	pushq	%rax
	movq	%ds,	%rax
	pushq	%rax

	movq	$0x10,	%rax
	movq	%rax,	%ds
	movq	%rax,	%es

	leaq	int_msg(%rip),	%rax			/* leaq get address */
	pushq	%rax
	movq	%rax,	%rdx
	movq	$0x00000000,	%rsi
	movq	$0x00ff0000,	%rdi
	movq	$0,	%rax
	callq	color_printk
	addq	$0x8,	%rsp

Loop:
	jmp	Loop	

	popq	%rax
	movq	%rax,	%ds
	popq	%rax
	movq	%rax,	%es

	popq	%r15
	popq	%r14
	popq	%r13
	popq	%r12
	popq	%r11
	popq	%r10
	popq	%r9
	popq	%r8

	popq	%rsi
	popq	%rdi
	popq	%rbp
	popq	%rdx
	popq	%rcx
	popq	%rbx
	popq	%rax
	iretq

int_msg:
	.asciz "Unknown interrupt or fault at RIP\n"

//=======	init page
.align 8

.org	0x1000

__PML4E:

	.quad	0x102007
	.fill	255,8,0
	.quad	0x102007
	.fill	255,8,0

.org	0x2000

__PDPTE:
	
	.quad	0x103003
	.fill	511,8,0

.org	0x3000

__PDE:

	.quad	0x000083	
	.quad	0x200083
	.quad	0x400083
	.quad	0x600083
	.quad	0x800083
	.quad	0xe0000083		/*0x a00000*/
	.quad	0xe0200083
	.quad	0xe0400083
	.quad	0xe0600083
	.quad	0xe0800083		/*0x1000000*/
	.quad	0xe0a00083
	.quad	0xe0c00083
	.quad	0xe0e00083
	.fill	499,8,0

//=======	GDT_Table

.section .data

.globl GDT_Table

GDT_Table:
	.quad	0x0000000000000000			/*0	NULL descriptor		       	00*/
	.quad	0x0020980000000000			/*1	KERNEL	Code	64-bit	Segment	08*/
	.quad	0x0000920000000000			/*2	KERNEL	Data	64-bit	Segment	10*/
	.quad	0x0020f80000000000			/*3	USER	Code	64-bit	Segment	18*/
	.quad	0x0000f20000000000			/*4	USER	Data	64-bit	Segment	20*/
	.quad	0x00cf9a000000ffff			/*5	KERNEL	Code	32-bit	Segment	28*/
	.quad	0x00cf92000000ffff			/*6	KERNEL	Data	32-bit	Segment	30*/
	.fill	10,8,0					/*8 ~ 9	TSS (jmp one segment <7>) in long-mode 128-bit 40*/
GDT_END:

GDT_POINTER:
GDT_LIMIT:	.word	GDT_END - GDT_Table - 1
GDT_BASE:	.quad	GDT_Table

//=======	IDT_Table

.globl IDT_Table

IDT_Table:
	.fill  512,8,0
IDT_END:

IDT_POINTER:
IDT_LIMIT:	.word	IDT_END - IDT_Table - 1
IDT_BASE:	.quad	IDT_Table

//=======	TSS64_Table

.globl	TSS64_Table

TSS64_Table:
	.fill  13,8,0
TSS64_END:


TSS64_POINTER:
TSS64_LIMIT:	.word	TSS64_END - TSS64_Table - 1
TSS64_BASE:	.quad	TSS64_Table
