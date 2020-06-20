#include "gate.h"

void set_interrupt_gate(unsigned int n, unsigned char ist, void *address)
{
    _set_gate(IDT_Table + n, 0x8E, ist, address); // P,DPL=0,TYPE=E
}

void set_trap_gate(unsigned int n, unsigned char ist, void *address)
{
    _set_gate(IDT_Table + n, 0x8F, ist, address); // P,DPL=0,TYPE=F
}

void set_system_gate(unsigned int n, unsigned char ist, void *address)
{
    _set_gate(IDT_Table + n, 0xEF, ist, address); // P,DPL=3,TYPE=F
}
