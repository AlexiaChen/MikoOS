#include "trap.h"

#include "gate.h"
#include "printk.h"

/*
The value of stack pointer register RSP (the value of RSP is stored in RDI
register as a parameter by exception handling module) is indexed up to 0x98
bytes to obtain the rip register value at the execution context of the
interrupted program and take it as the address of generating exception
instruction.
*/

/*
error code format: 32-bit

|----(31 to 16 bit)------|-------(15 to 3 bit)------|--(2 bit)--|--(1
bit)--|--(0 bit)--|

31-61 bit: reserved
15-3 bit: segment selector
2 bit: TI, This bit is set only when the IDT flag bit is unset. If this bit is
set, the segment selector of the error code records the segment descriptor or
gate descriptor in the local descriptor table (LDT). This flag unset means that
it records the global descriptor table (GDT) descriptor 1 bit: IDT, If this bit
is set, the segment selector of the error code records the gate descriptor in
the interrupt descriptor table(IDT), and unset that it records the descriptor in
the descriptor table (GDT / LDT) 0 bit: EXT, If this bit is set, the exception
is triggered during the delivery of an external event to the program
*/

#define EXT_FLAG 0x01 // 1 << 0
#define IDT_FLAG 0x02 // 1 << 1
#define TI_FLAG 0x04  // 1 << 2

static bool_t
is_flag_set(unsigned long error_code, int flag);
static bool_t
is_flag_set(unsigned long error_code, int flag)
{
    return ((error_code & flag) == 1) ? true : false;
}

static void
do_print_trap_context(const char* name,
                      unsigned long rsp,
                      unsigned long error_code);
static void
do_print_trap_context(const char* name,
                      unsigned long rsp,
                      unsigned long error_code)
{
    unsigned long* p = NULL;
    // RIP = 0x98
    p = (unsigned long*)(rsp + 0x98);
    color_printk(RED,
                 BLACK,
                 "%s,ERROR_CODE:%#018lx,RSP:%#018lx,RIP:%#018lx\n",
                 name,
                 error_code,
                 rsp,
                 *p);
}

void do_divide_error(unsigned long rsp, unsigned long error_code);
void do_divide_error(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_divide_error(0)", rsp, error_code);
    while (true)
        ;
}

void do_debug(unsigned long rsp, unsigned long error_code);
void do_debug(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_debug(1)", rsp, error_code);
    while (true)
        ;
}

void do_nmi(unsigned long rsp, unsigned long error_code);
void do_nmi(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_nmi(2)", rsp, error_code);
    while (true)
        ;
}

void do_int3(unsigned long rsp, unsigned long error_code);
void do_int3(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_int3(3)", rsp, error_code);
    while (true)
        ;
}

void do_overflow(unsigned long rsp, unsigned long error_code);
void do_overflow(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_overflow(4)", rsp, error_code);
    while (true)
        ;
}

void do_bounds(unsigned long rsp, unsigned long error_code);
void do_bounds(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_bounds(5)", rsp, error_code);
    while (true)
        ;
}

void do_undefined_opcode(unsigned long rsp, unsigned long error_code);
void do_undefined_opcode(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_undefined_opcode(6)", rsp, error_code);
    while (true)
        ;
}

void do_dev_not_available(unsigned long rsp, unsigned long error_code);
void do_dev_not_available(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_dev_not_available(7)", rsp, error_code);
    while (true)
        ;
}

void do_double_fault(unsigned long rsp, unsigned long error_code);
void do_double_fault(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_double_fault(8)", rsp, error_code);
    while (true)
        ;
}

void do_coprocessor_segment_overrun(unsigned long rsp, unsigned long error_code);
void do_coprocessor_segment_overrun(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_coprocessor_segment_overrun(9)", rsp, error_code);
    while (true)
        ;
}

void do_invalid_TSS(unsigned long rsp, unsigned long error_code);
void do_invalid_TSS(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_invalid_TSS(10)", rsp, error_code);

    if (is_flag_set(error_code, EXT_FLAG))
        color_printk(
            RED,
            BLACK,
            "The exception occurred during delivery of an event external to the "
            "program,such as an interrupt or an earlier exception.\n");

    if (is_flag_set(error_code, IDT_FLAG))
        color_printk(RED, BLACK, "Refers to a gate descriptor in the IDT;\n");
    else
        color_printk(
            RED, BLACK, "Refers to a descriptor in the GDT or the current LDT;\n");

    if (!is_flag_set(error_code, IDT_FLAG))
    {
        if (is_flag_set(error_code, TI_FLAG))
            color_printk(
                RED, BLACK, "Refers to a segment or gate descriptor in the LDT;\n");
        else
            color_printk(RED, BLACK, "Refers to a descriptor in the current GDT;\n");
    }

    color_printk(
        RED, BLACK, "Segment Selector Index:%#010x\n", error_code & 0xfff8);

    while (true)
        ;
}

void do_segment_not_present(unsigned long rsp, unsigned long error_code);
void do_segment_not_present(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_segment_not_present(11)", rsp, error_code);

    if (is_flag_set(error_code, EXT_FLAG))
        color_printk(
            RED,
            BLACK,
            "The exception occurred during delivery of an event external to the "
            "program,such as an interrupt or an earlier exception.\n");

    if (is_flag_set(error_code, IDT_FLAG))
        color_printk(RED, BLACK, "Refers to a gate descriptor in the IDT;\n");
    else
        color_printk(
            RED, BLACK, "Refers to a descriptor in the GDT or the current LDT;\n");

    if (!is_flag_set(error_code, IDT_FLAG))
    {
        if (is_flag_set(error_code, TI_FLAG))
            color_printk(
                RED, BLACK, "Refers to a segment or gate descriptor in the LDT;\n");
        else
            color_printk(RED, BLACK, "Refers to a descriptor in the current GDT;\n");
    }

    color_printk(
        RED, BLACK, "Segment Selector Index:%#010x\n", error_code & 0xfff8);

    while (true)
        ;
}

void do_stack_segment_fault(unsigned long rsp, unsigned long error_code);
void do_stack_segment_fault(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_stack_segment_fault(12)", rsp, error_code);

    if (is_flag_set(error_code, EXT_FLAG))
        color_printk(
            RED,
            BLACK,
            "The exception occurred during delivery of an event external to the "
            "program,such as an interrupt or an earlier exception.\n");

    if (is_flag_set(error_code, IDT_FLAG))
        color_printk(RED, BLACK, "Refers to a gate descriptor in the IDT;\n");
    else
        color_printk(
            RED, BLACK, "Refers to a descriptor in the GDT or the current LDT;\n");

    if (!is_flag_set(error_code, IDT_FLAG))
    {
        if (is_flag_set(error_code, TI_FLAG))
            color_printk(
                RED, BLACK, "Refers to a segment or gate descriptor in the LDT;\n");
        else
            color_printk(RED, BLACK, "Refers to a descriptor in the current GDT;\n");
    }

    color_printk(
        RED, BLACK, "Segment Selector Index:%#010x\n", error_code & 0xfff8);

    while (true)
        ;
}

void do_general_protection(unsigned long rsp, unsigned long error_code);
void do_general_protection(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_general_protection(13)", rsp, error_code);

    if (is_flag_set(error_code, EXT_FLAG))
        color_printk(
            RED,
            BLACK,
            "The exception occurred during delivery of an event external to the "
            "program,such as an interrupt or an earlier exception.\n");

    if (is_flag_set(error_code, IDT_FLAG))
        color_printk(RED, BLACK, "Refers to a gate descriptor in the IDT;\n");
    else
        color_printk(
            RED, BLACK, "Refers to a descriptor in the GDT or the current LDT;\n");

    if (!is_flag_set(error_code, IDT_FLAG))
    {
        if (is_flag_set(error_code, TI_FLAG))
            color_printk(
                RED, BLACK, "Refers to a segment or gate descriptor in the LDT;\n");
        else
            color_printk(RED, BLACK, "Refers to a descriptor in the current GDT;\n");
    }

    color_printk(
        RED, BLACK, "Segment Selector Index:%#010x\n", error_code & 0xfff8);

    while (true)
        ;
}

/*
page error code format: 32-bit

|----------(31 to 5 bit)----------|--(4 bit)--|--(3 bit)--|--(2 bit)--|--(1
bit)--|--(0 bit)--|

31 to 5 bit: Reserved
4 bit:  I/D, if the flag is unset means that no exception was thrown while
getting the instruction, if flag is set, means that have exception was thrown
while getting the instruction 3 bit: RSVD, if the flag is unset means that the
reserved bit of the page table entry did not throw an exception, if flag set,
means that the reserved bit of the page table entry being set  cause to  throw
an exception 2 bit: U/S, if the flag is unset means that using super user rights
to access the page throw an exception if flag set means that using normal user
rights to access the page throw an exception 1 bit: W/R, if the flag is unset
means that read page, and the flag is set means that write page 0 bit: P, if the
flag is unset means that the page is not present, and the flag is set means that
page protection
*/

#define P_FLAG 0x01    // 1 << 0
#define WR_FLAG 0x02   // 1 << 1
#define US_FLAG 0x04   // 1 << 2
#define RSVD_FLAG 0x08 // 1 << 3
#define ID_FLAG 0x10   // 1 << 4

static bool_t
is_page_flag_set(unsigned long error_code, int flag);
static bool_t
is_page_flag_set(unsigned long error_code, int flag)
{
    return ((error_code & flag) > 0) ? true : false;
}

void do_page_fault(unsigned long rsp, unsigned long error_code);
void do_page_fault(unsigned long rsp, unsigned long error_code)
{
    unsigned long* p = NULL;
    unsigned long cr2 = 0;

    __asm__ __volatile__("movq	%%cr2,	%0"
                         : "=r"(cr2)::"memory");

    p = (unsigned long*)(rsp + 0x98);
    color_printk(RED,
                 BLACK,
                 "do_page_fault(14),ERROR_CODE:%#018lx,RSP:%#018lx,RIP:%#018lx\n",
                 error_code,
                 rsp,
                 *p);

    if (!is_page_flag_set(error_code, P_FLAG))
        color_printk(RED, BLACK, "Page Not-Present,\t");

    if (is_page_flag_set(error_code, WR_FLAG))
        color_printk(RED, BLACK, "Write Cause Fault,\t");
    else
        color_printk(RED, BLACK, "Read Cause Fault,\t");

    if (is_page_flag_set(error_code, US_FLAG))
        color_printk(RED, BLACK, "Fault in user(3)\t");
    else
        color_printk(RED, BLACK, "Fault in supervisor(0,1,2)\t");

    if (is_page_flag_set(error_code, RSVD_FLAG))
        color_printk(RED, BLACK, ",Reserved Bit Cause Fault\t");

    if (is_page_flag_set(error_code, IDT_FLAG))
        color_printk(RED, BLACK, ",Instruction fetch Cause Fault");

    color_printk(RED, BLACK, "\n");

    color_printk(RED, BLACK, "CR2:%#018lx\n", cr2);

    while (true)
        ;
}

void do_x87_FPU_error(unsigned long rsp, unsigned long error_code);
void do_x87_FPU_error(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_x87_FPU_error(16)", rsp, error_code);
    while (true)
        ;
}

void do_alignment_check(unsigned long rsp, unsigned long error_code);
void do_alignment_check(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_alignment_check(17)", rsp, error_code);
    while (true)
        ;
}

void do_machine_check(unsigned long rsp, unsigned long error_code);
void do_machine_check(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_machine_check(18)", rsp, error_code);
    while (true)
        ;
}

void do_SIMD_exception(unsigned long rsp, unsigned long error_code);
void do_SIMD_exception(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_SIMD_exception(19)", rsp, error_code);
    while (true)
        ;
}

void do_virtualization_exception(unsigned long rsp, unsigned long error_code);
void do_virtualization_exception(unsigned long rsp, unsigned long error_code)
{
    do_print_trap_context("do_virtualization_exception(20)", rsp, error_code);
    while (true)
        ;
}

void sys_vector_handler_init(void);
void sys_vector_handler_init()
{
    set_trap_gate(0, 1, divide_error);
    set_trap_gate(1, 1, debug);
    set_interrupt_gate(2, 1, nmi);
    set_system_gate(3, 1, int3);
    set_system_gate(4, 1, overflow);
    set_system_gate(5, 1, bounds);
    set_trap_gate(6, 1, undefined_opcode);
    set_trap_gate(7, 1, dev_not_available);
    set_trap_gate(8, 1, double_fault);
    set_trap_gate(9, 1, coprocessor_segment_overrun);
    set_trap_gate(10, 1, invalid_TSS);
    set_trap_gate(11, 1, segment_not_present);
    set_trap_gate(12, 1, stack_segment_fault);
    set_trap_gate(13, 1, general_protection);
    set_trap_gate(14, 1, page_fault);
    // 15 Intel reserved. Do not use.
    set_trap_gate(16, 1, x87_FPU_error);
    set_trap_gate(17, 1, alignment_check);
    set_trap_gate(18, 1, machine_check);
    set_trap_gate(19, 1, SIMD_exception);
    set_trap_gate(20, 1, virtualization_exception);

    // set_system_gate(SYSTEM_CALL_VECTOR,7,system_call);
}
