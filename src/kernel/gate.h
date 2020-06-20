#ifndef GATE_H
#define GATE_H

#include "util.h"

struct desc_struct 
{
	unsigned char x[8];
};

struct gate_struct
{
	unsigned char x[16];
};

extern struct desc_struct GDT_Table[];
extern struct gate_struct IDT_Table[];
extern unsigned int TSS64_Table[26];

/*
Initialize each table item in the IDT, and configure the descriptor as interrupt gate and trap gate with DPL = 0 or trap gate with DPL = 3 according to the exception type
*/

void set_interrupt_gate(unsigned int n, unsigned char ist, void *address);

void set_trap_gate(unsigned int n, unsigned char ist, void *address);

void set_system_gate(unsigned int n, unsigned char ist, void *address);

/*
functions set_interrupt_gate, set_trap_gate, set_system_gate

The above three descriptor configuration functions use _set_gate function initializes the table entries in the IDT. 
This _set_gate function parameter is IDT_Table is the identifier that global IDT declared in the kernel execution header file head.S IDT_Table

*/

#define _set_gate(gate_selector_address,attribute,ist,code_address)	\
do								\
{	unsigned long __d0,__d1;				\
	__asm__ __volatile__	(	"movw	%%dx,	%%ax	\n\t"	\
					"andq	$0x7,	%%rcx	\n\t"	\
					"addq	%4,	%%rcx	\n\t"	\
					"shlq	$32,	%%rcx	\n\t"	\
					"addq	%%rcx,	%%rax	\n\t"	\
					"xorq	%%rcx,	%%rcx	\n\t"	\
					"movl	%%edx,	%%ecx	\n\t"	\
					"shrq	$16,	%%rcx	\n\t"	\
					"shlq	$48,	%%rcx	\n\t"	\
					"addq	%%rcx,	%%rax	\n\t"	\
					"movq	%%rax,	%0	\n\t"	\
					"shrq	$32,	%%rdx	\n\t"	\
					"movq	%%rdx,	%1	\n\t"	\
					:"=m"(*((unsigned long *)(gate_selector_address)))	,					\
					 "=m"(*(1 + (unsigned long *)(gate_selector_address))),"=&a"(__d0),"=&d"(__d1)		\
					:"i"(attribute << 8),									\
					 "3"((unsigned long *)(code_address)),"2"(0x8 << 16),"c"(ist)				\
					:"memory"		\
				);				\
}while(false)


#endif