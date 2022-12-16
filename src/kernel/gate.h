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

enum GateAttribute
{
  GATE_INTERRUPT = 0x8E,
  GATE_TRAP = 0x8F,
  GATE_SYSTEM = 0xEF,
  GATE_SYSTEM_INTERRUPT = 0xEE
};

/*
Initialize each table item in the IDT, and configure the descriptor as interrupt
gate and trap gate with DPL = 0 or trap gate with DPL = 3 according to the
exception type
*/

// parameter n is the index of the IDT table
void
set_interrupt_gate(unsigned int n, unsigned char ist, void* address);

void
set_trap_gate(unsigned int n, unsigned char ist, void* address);

void
set_system_gate(unsigned int n, unsigned char ist, void* address);

// int 3
void
set_system_interrupt_gate(unsigned int n, unsigned char ist, void* address);

/*
functions set_interrupt_gate, set_trap_gate, set_system_gate

The above three descriptor configuration functions use _set_gate function
initializes the table entries in the IDT. This _set_gate function parameter is
IDT_Table is the identifier that global IDT declared in the kernel execution
header file head.S IDT_Table

*/

#define _set_gate(gate_selector_addr, attr, ist, code_addr)                    \
  do {                                                                         \
    unsigned long __d0, __d1;                                                  \
    __asm__ __volatile__("movw	%%dx,	%%ax	\n\t"                                \
                         "andq	$0x7,	%%rcx	\n\t"                               \
                         "addq	%4,	%%rcx	\n\t"                                 \
                         "shlq	$32,	%%rcx	\n\t"                                \
                         "addq	%%rcx,	%%rax	\n\t"                              \
                         "xorq	%%rcx,	%%rcx	\n\t"                              \
                         "movl	%%edx,	%%ecx	\n\t"                              \
                         "shrq	$16,	%%rcx	\n\t"                                \
                         "shlq	$48,	%%rcx	\n\t"                                \
                         "addq	%%rcx,	%%rax	\n\t"                              \
                         "movq	%%rax,	%0	\n\t"                                 \
                         "shrq	$32,	%%rdx	\n\t"                                \
                         "movq	%%rdx,	%1	\n\t"                                 \
                         : "=m"(*((unsigned long*)(gate_selector_addr))),      \
                           "=m"(*(1 + (unsigned long*)(gate_selector_addr))),  \
                           "=&a"(__d0),                                        \
                           "=&d"(__d1)                                         \
                         : "i"(attr << 8),                                     \
                           "3"((unsigned long*)(code_addr)),                   \
                           "2"(0x8 << 16),                                     \
                           "c"(ist)                                            \
                         : "memory");                                          \
  } while (0)

/*
Load the segment selector of TSS segment descriptor to TR rigester
*/
#define load_TR(n)                                                             \
  do {                                                                         \
    __asm__ __volatile__("ltr	%%ax" : : "a"(n << 3) : "memory");             \
  } while (0)

/*
set every RSP entries and IST entries in TSS segment
*/
void
set_tss64(unsigned long rsp0,
          unsigned long rsp1,
          unsigned long rsp2,
          unsigned long ist1,
          unsigned long ist2,
          unsigned long ist3,
          unsigned long ist4,
          unsigned long ist5,
          unsigned long ist6,
          unsigned long ist7);

#endif