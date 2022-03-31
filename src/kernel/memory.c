#include "memory.h"
#include "printk.h"
// However, since 7E00h is a physical address, 
// it must be mapped through the page table before it can be used by the program. 
// The converted linear address is 0xffff800000007e00h, 
// which is the target address for the program to operate
void init_memory()
{
  int i, j;
  unsigned long total_mem = 0;
  struct MemoryE820Format *p = NULL;
  color_printk(BLUE,BLACK,"Display Physics Address MAP,Type(1:RAM,2:ROM or Reserved,3:ACPI Reclaim Memory,4:ACPI NVS Memory,Others:Undefine)\n");

  p = (struct MemoryE820Format*)0xffff800000007e00;

  for(i = 0; i < 32; ++i)
  {
     color_printk(ORANGE,BLACK,"Address:%#010x,%08x\tLength:%#010x,%08x\tType:%#010x\n", p->address2, p->address1, p->length2, p->length1, p->type);
     unsigned int temp = 0;
     // type = 1 is useable physical memory space
     // the usable physical memory space capacity is combined of two parts:
     // part one: capacity is 0x9f000
     // part two: capacity is 0x7fef0000
     // total is 0x7ff8f000 Byte near 2GB
     if(p->type == 1)
     {
       temp = p->length2;
       total_mem += p->length1;
       total_mem += temp << 32;
     }

     p++;
     // normally, type is cannot greater than 4, if it is, program must meet junk data while running
     if(p->type > 4)
     {
       break;
     }
  }

  color_printk(ORANGE,BLACK,"OS Can Used Total RAM:%#018lx\n",total_mem);
}
