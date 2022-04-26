#ifndef __MEMORY_H__
#define __MEMORY_H__

#include "util.h"

// 8 Bytes per cell, the number of bytes per page table entry in 64-bit mode is expanded from 4 bytes to 8 bytes
// one page table is 4KB  4KB / 8 Byte = 512
// 512 is number of entries of a page table
const unsigned int PTRS_PER_PAGE = 512;

// represents the starting linear address of the kernel layer, 
// which is located at physical address 0 (this value must be remapped through the page table)
const unsigned long PAGE_OFFSET = 0xffff800000000000;

const unsigned int PAGE_GDT_SHIFT = 39;

// They represent the physical page capacity for each page table entry in 64-bit mode
// 2^30 Bytes = 2^30 * 2^0 = 1GB
const unsigned int PAGE_1G_SHIFT = 30;
// 2^21 Bytes = 2^20 * 2^1 = 2MB
const unsigned int PAGE_2M_SHIFT = 21;
// 2^12 Bytes = 2^10 * 2^2 = 4KB
const unsigned int PAGE_4K_SHIFT = 12;

const unsigned long PAGE_2M_SIZE = (1UL << PAGE_2M_SHIFT);
const unsigned long PAGE_4K_SIZE = (1UL << PAGE_4K_SHIFT);

// 1 << 21 =  100000...000000 (22-bits) - 1 = 01111...111111 (22-bits). and all of 1 bits, it cover under 2MB addr value 
const unsigned long PAGE_2M_MASK = (~ (PAGE_2M_SIZE - 1));
// 1 << 12 =  100000...000000 (13-bits) - 1 = 01111...111111 (13-bits). and all of 1 bits, it cover under 4KB addr value 
const unsigned long PAGE_4K_MASK = (~ (PAGE_4K_SIZE - 1));


// The function PAGE_2M_ALIGN(addr) is used to set the parameter addr to the upper boundary of the 2 MB page pair
unsigned long PAGE_2M_UPPER_ALIGN(unsigned long addr);
unsigned long PAGE_4K_UPPER_ALIGN(unsigned long addr);

// The function PAGE_2M_LOWER_ALIGN(addr) is used to set the parameter addr to the lower boundary of the 2 MB page pair
unsigned long PAGE_2M_LOWER_ALIGN(unsigned long addr);

// The inline function VirtualToPhysicalAddr(addr) is used to convert the kernel level virtual address to physical address, 
// please note that this function is conditional, currently only the first 10 MB of the physical address is mapped to 
// the linear address 0xffff800000000000 (in the page table defined in the head.S file), and only this 10 MB of memory space is available 
// for this  function
unsigned long VirtualToPhysicalAddr(unsigned long addr);
unsigned long* PhysicalToVirtualAddr(unsigned long addr);

enum E820EntryType
{
  E820_RAM = 1, // Usable (normal) RAM
  E820_RESERVED = 2, // Reserved - unusable
  E820_ACPI_RECLAIM = 3, // ACPI reclaimable memory
  E820_ACPI_NVS = 4, // ACPI NVS memory
  E820_BAD_MEMORY = 5, // Area containing bad memory
  E820_UNDEFINED = 0
};

// There are several groups of physical address space information at address 7E00h, 
// which describe the address space division of the computer platform, 
// the number of which will depend on the current motherboard hardware configuration and physical memory capacity information, 
// each physical address space information occupies 20 Byte
struct MemoryE820Format
{
  unsigned int address1;
  unsigned int address2;
  unsigned int length1;
  unsigned int length2;
  unsigned int type;
};

// The special attribute __attribute__((packed)) in the structure; 
// modifying the structure will not generate the alignment space, and instead use the compact format, 
// which is the only way to correctly index the memory space distribution information at 
// the linear address 0xffff800000007e00 from the struct E820 structure
struct E820Entry
{
  unsigned long address;
  unsigned long length;
  unsigned int  type;
}__attribute__((packed));

#define E820_MAX_ENTRIES 32

// Saving all information about memory for use by memory management modules
// E820 https://en.wikipedia.org/wiki/E820
// https://wiki.osdev.org/Detecting_Memory_(x86)
struct GlobalE820Table
{
  struct E820Entry entries[E820_MAX_ENTRIES]; // Physical memory segments array
  unsigned long   number_entries; // Number of physical memory segments
};

struct GlobalMemoryDescriptor
{
  struct GlobalE820Table e820;

  unsigned long   bits_map; // Physical space page(struct page) bitmap (address)
  unsigned long   bits_size;  // number of pages in the physical space
  unsigned long   bits_length; // Number of bytes in the bitmap

  struct Page* pages; // Physical space page array
  unsigned long   pages_size; // number of struct page
  unsigned long   pages_length; // length of struct page array

  struct Zone* zones; //  zones array
  unsigned long   zones_size; // number of struct zone
  unsigned long   zones_length; // length of struct zone array

  unsigned long  start_kernel_code; // start address of kernel code segment
  unsigned long  end__kernel_code;  // end address of kernel code segment
  unsigned long  end_data; // end address of kernel data segment
  unsigned long  end_brk; // end address of kernel (BSS segmengt)

  unsigned long  end_of_struct; // end address of struct GlobalMemoryDescriptor
};

// every 2M physical page is represent a page
struct Page
{
  struct Zone *zone;          // pointer to the zone that the page belongs to
  unsigned long physical_address; // physical address of the page
  unsigned long attribute; // page attribute (mapping state of the page, active state, user info etc )

  unsigned long reference_count; // reference count of the page
  unsigned long create_time; // the time when the page is created
};

struct Zone
{
  struct Page*   pages; // pointer to the first page of the zone
  unsigned long  number_pages; // number of pages in the zone

  unsigned long  start_address; // start address of the zone
  unsigned long  end_address; // end address of the zone
  unsigned long  attribute; // zone attribute （Describes whether the current zone supports DMA, whether the pages are mapped through the page table, and other information.）
  unsigned long  length; //  length of the zone align by page size

  struct GlobalE820Table* e820_table; // pointer to the global e820 table

  unsigned long page_using_count; // number of pages using in the zone
  unsigned long page_free_count; // number of pages free in the zone

  // Since the mapping of physical pages in the page table can be a one-to-many 
  // relationship, a physical page can be mapped to multiple locations in the linear 
  // address space at the same time, so the member variables total_pages_ref_count and 
  // page_using_count are not necessarily equal in value
  unsigned long total_pages_ref_count; // all of  pages reference count
};

extern struct GlobalMemoryDescriptor global_memory_descriptor;

void init_memory();

#endif
