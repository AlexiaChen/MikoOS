#include "memory.h"
#include "printk.h"

unsigned long PAGE_2M_UPPER_ALIGN(unsigned long addr) 
{
  return ((addr + PAGE_2M_SIZE - 1) & PAGE_2M_MASK);
}

unsigned long PAGE_4K_UPPER_ALIGN(unsigned long addr) 
{
  return ((addr + PAGE_4K_SIZE - 1) & PAGE_4K_MASK);
}

unsigned long PAGE_2M_LOWER_ALIGN(unsigned long addr) 
{
  return (addr >> PAGE_2M_SHIFT);
}

unsigned long VirtualToPhysicalAddr(unsigned long addr)
{
  return addr - PAGE_OFFSET; 
}

unsigned long* PhysicalToVirtualAddr(unsigned long addr)
{
  return (unsigned long*)(addr + PAGE_OFFSET);
}

// However, since 7E00h is a physical address, 
// it must be mapped through the page table before it can be used by the program. 
// The converted linear address is 0xffff800000007e00h, 
// which is the target address for the program to operate
void init_memory()
{
  int i, j;
  unsigned long total_mem = 0;
  struct E820Entry *p = NULL;
  color_printk(BLUE,BLACK,"Display Physics Address MAP,Type(1:RAM,2:ROM or Reserved,3:ACPI Reclaim Memory,4:ACPI NVS Memory,Others:Undefine)\n");

  p = (struct E820Entry*)0xffff800000007e00;

  for(i = 0; i < E820_MAX_ENTRIES; ++i)
  {
     color_printk(ORANGE,BLACK,"Address:%#018lx\tLength:%#018lx\tType:%#010x\n",p->address, p->length, p->type);
     unsigned long temp = 0;
     // type = 1 is useable physical memory space
     // the usable physical memory space capacity is combined of two parts:
     // part one: capacity is 0x9f000
     // part two: capacity is 0x7fef0000
     // total is 0x7ff8f000 Byte near 2GB  （megs: 2048 see conf folder linux-bochsrc）
     if(p->type == E820_RAM)
     {
       total_mem += p->length;
     }

     global_memory_descriptor.e820.entries[i].address += p->address;
     global_memory_descriptor.e820.entries[i].length += p->length;
     global_memory_descriptor.e820.entries[i].type = p->type;
     global_memory_descriptor.e820.number_entries = i;

     p++;
     // normally, type is cannot greater than 4, if it is, program must meet junk data while running
     if(p->type > E820_ACPI_NVS || p->length == 0 || p->type < E820_RAM)
     {
       color_printk(RED,BLACK,"Junk data in E820 table\n");
       break;
     }
  }

  color_printk(ORANGE,BLACK,"OS Can Used Total RAM:%#018lx\n",total_mem);

  
  // After detecting the available physical memory segments, 
  // the start address of these segments is aligned to the upper boundary of the 2 MB page 
  // using the macro function PAGE_2M_ALIGN, and the aligned address is the valid memory start address of the segment. 
  // The end address of these segments is obtained by adding the original start address of the segment and the segment length, 
  // and then the result of the calculation is aligned by shifting to the lower boundary of the 2 MB page, 
  // or by using the previously defined macro constant PAGE_2M_MASK to align the 
  // lower boundary of the page. If the calculated start address is less than or equal 
  // to the calculated end address, the segment is considered a valid memory segment, 
  // and the number of available physical pages is calculated and the total number of 
  // available physical pages is printed on the screen.
  unsigned long total_pages = 0;
  for(i = 0; i < global_memory_descriptor.e820.number_entries; ++i)
  {
     unsigned long start, end;
     // usable physical memory space
     if(global_memory_descriptor.e820.entries[i].type == E820_RAM)
     {
       start = PAGE_2M_UPPER_ALIGN(global_memory_descriptor.e820.entries[i].address);
       end = PAGE_2M_LOWER_ALIGN(global_memory_descriptor.e820.entries[i].address + global_memory_descriptor.e820.entries[i].length) << PAGE_2M_SHIFT;
      if(end > start)
      {
        total_pages += PAGE_2M_LOWER_ALIGN(end - start);
      }      
     }
  }

  color_printk(ORANGE,BLACK,"OS Can Used Total 2M Pages:%#010x=%010d\n",total_pages, total_pages);

}
