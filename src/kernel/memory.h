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


// The inline function PAGE_2M_ALIGN(addr) is used to set the parameter addr to the upper boundary of the 2 MB page pair
unsigned long PAGE_2M_ALIGN(unsigned long addr);
unsigned long PAGE_4K_ALIGN(unsigned long addr);

// The inline function VirtualToPhysicalAddr(addr) is used to convert the kernel level virtual address to physical address, 
// please note that this function is conditional, currently only the first 10 MB of the physical address is mapped to 
// the linear address 0xffff800000000000 (in the page table defined in the head.S file), and only this 10 MB of memory space is available 
// for this  function
unsigned long VirtualToPhysicalAddr(unsigned long addr);
unsigned long* PhysicalToVirtualAddr(unsigned long addr);

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
struct E820
{
  unsigned long address;
  unsigned long length;
  unsigned int  type;
}__attribute__((packed));


// Saving all information about memory for use by memory management modules
struct GlobalMemoryDescriptor
{
  struct E820     e820[32];
  unsigned long   e820_length;
};

extern struct GlobalMemoryDescriptor memory_management_struct;

void init_memory();

#endif
