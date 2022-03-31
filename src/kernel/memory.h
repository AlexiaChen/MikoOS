#include "util.h"

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
