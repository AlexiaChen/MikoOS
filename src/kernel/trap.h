#ifndef TRAP_H
#define TRAP_H

void divide_error(void);
void debug(void);
void nmi(void);
void int3(void);
void overflow(void);
void bounds(void);
void undefined_opcode(void);
void dev_not_available(void);
void double_fault(void);
void coprocessor_segment_overrun(void);
void invalid_TSS(void);
void segment_not_present(void);
void stack_segment_fault(void);
void general_protection(void);
void page_fault(void);
void x87_FPU_error(void);
void alignment_check(void);
void machine_check(void);
void SIMD_exception(void);
void virtualization_exception(void);

/*init IDT*/
void sys_vector_handler_init(void);

#endif