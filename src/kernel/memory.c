#include "memory.h"
#include "printk.h"
#include "util.h"

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

unsigned long BYTES_NUM_UPPER_ALIGN(unsigned long size, unsigned long align_bytes)
{
  const unsigned long BYTES_SIZE = align_bytes;
  const unsigned long BYTES_MASK = (~ (BYTES_SIZE - 1));
  return ((size + BYTES_SIZE - 1) & BYTES_MASK);
}

unsigned long BITS_NUM_UPPER_ALIGN(unsigned long size, unsigned long align_bytes)
{
  const unsigned long BYTES_SIZE = align_bytes;
  const unsigned long BYTES_MASK = (~ (BYTES_SIZE - 1));
  const unsigned long bytes_size = (size + align_bytes*8 - 1) / 8;
  return (bytes_size & BYTES_MASK);
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
  unsigned long total_mem = 0;
  struct E820Entry *p = NULL;
  color_printk(BLUE,BLACK,"Display Physics Address MAP,Type(1:RAM,2:ROM or Reserved,3:ACPI Reclaim Memory,4:ACPI NVS Memory,Others:Undefine)\n");

  p = (struct E820Entry*)0xffff800000007e00;

  for(int i = 0; i < E820_MAX_ENTRIES; ++i)
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

     global_memory_descriptor.e820.entries[i].address = p->address;
     global_memory_descriptor.e820.entries[i].length = p->length;
     global_memory_descriptor.e820.entries[i].type = p->type;
     global_memory_descriptor.e820.number_entries = i + 1;

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
  for(int i = 0; i < global_memory_descriptor.e820.number_entries; ++i)
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

  
  // |  -----  physical memory space for pages (ROM or void memory pages etc)  -----  |   -- isolated space --    | bitmap 
  unsigned long end_addr_of_physical_space = 
    global_memory_descriptor.e820.entries[global_memory_descriptor.e820.number_entries - 1].address 
    + global_memory_descriptor.e820.entries[global_memory_descriptor.e820.number_entries - 1].length;

  // bits map pointer, pointer to end address of kernel
  // This is done to preserve a small section of isolated space to prevent misuse from corrupting data in other spaces (this is why  upper alginment needed)
  global_memory_descriptor.bits_map = (unsigned long*)PAGE_4K_UPPER_ALIGN(global_memory_descriptor.end_brk);
  // number of avaible 2M pages
  // This physical address space avaible pages that include not only available physical memory, 
  // but also memory voids and ROM address space
  global_memory_descriptor.bits_size = PAGE_2M_LOWER_ALIGN(end_addr_of_physical_space);
  
  static const unsigned long BITS_PER_BYTE = 8;
  static const unsigned long LONG_TYPE_BYTES = sizeof(long);
  static const unsigned long LONG_TYPE_BITS = LONG_TYPE_BYTES * BITS_PER_BYTE;
  global_memory_descriptor.bits_length = BITS_NUM_UPPER_ALIGN(global_memory_descriptor.bits_size, LONG_TYPE_BYTES);
  // The entire bits_map space is set all the way to mark non-memory pages (memory voids and ROM space) as used, 
  // and then the available physical memory pages in the map bitmap are programmatically reset later.
  memset(global_memory_descriptor.bits_map, 0xff, global_memory_descriptor.bits_length);

  // | bitmap | pages |
  // Create the storage space and allocation records for the page array. struct page is located after the bitmap, 
  // and the number of elements in the array is the number of pages that can be allocated in the physical address space, 
  // which is allocated and calculated in a similar way to the bitmap, except that the array is zeroed out for subsequent initialization procedures.
  unsigned long pages_addr = (unsigned long)(global_memory_descriptor.bits_map + global_memory_descriptor.bits_length);
  global_memory_descriptor.pages = (struct Page*)PAGE_4K_UPPER_ALIGN(pages_addr);
  global_memory_descriptor.pages_size = PAGE_2M_LOWER_ALIGN(end_addr_of_physical_space);

  static const unsigned long PER_PAGE_BYTES = sizeof(struct Page);
  global_memory_descriptor.pages_length =  BYTES_NUM_UPPER_ALIGN(global_memory_descriptor.pages_size * PER_PAGE_BYTES, LONG_TYPE_BYTES);
  memset(global_memory_descriptor.pages, 0x00, global_memory_descriptor.pages_length);

  // | pages | zones |  
  // Same as above, however, since the number of elements in the struct zone array cannot be calculated for the time being, 
  // the value is assigned to 0 for the time being, and the zones_length is calculated according to the 5 struct zone structures for the time being
  unsigned long zones_addr = (unsigned long)(global_memory_descriptor.pages + global_memory_descriptor.pages_length);
  global_memory_descriptor.zones = (struct Zone*)PAGE_4K_UPPER_ALIGN(zones_addr);
  global_memory_descriptor.zones_size = 0;
  const unsigned long INIT_ZONES_NUMBER = 5;
  static const unsigned long PER_ZONE_BYTES = sizeof(struct Zone);
  global_memory_descriptor.zones_length = BYTES_NUM_UPPER_ALIGN(INIT_ZONES_NUMBER * PER_ZONE_BYTES, LONG_TYPE_BYTES);

}
