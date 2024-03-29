#include "memory.h"

#include "printk.h"
#include "util.h"

// >>> PAGE_4BYTES_SHIFT = 2
// >>> PAGE_4BYTES_SIZE = (1 << PAGE_4BYTES_SHIFT)
// >>> PAGE_4BYTES_MASK = (~ (PAGE_4BYTES_SIZE - 1))
// >>> (7 + PAGE_4BYTES_SIZE - 1) & PAGE_4BYTES_MASK
// 8
// >>> (8 + PAGE_4BYTES_SIZE - 1) & PAGE_4BYTES_MASK
// 8
// >>> (9 + PAGE_4BYTES_SIZE - 1) & PAGE_4BYTES_MASK
// 12
unsigned long
PAGE_2M_UPPER_ALIGN(unsigned long addr)
{
    return ((addr + PAGE_2M_SIZE - 1) & PAGE_2M_MASK);
}

unsigned long
PAGE_4K_UPPER_ALIGN(unsigned long addr)
{
    return ((addr + PAGE_4K_SIZE - 1) & PAGE_4K_MASK);
}

// PAGE_4BYTES_SHIFT = 2
// >>> 7 >> PAGE_4BYTES_SHIFT
// 1
// >>> 9 >> PAGE_4BYTES_SHIFT
// 2
// >>> 7 >> PAGE_4BYTES_SHIFT
// 1
// >>> 1 << PAGE_4BYTES_SHIFT
// 4
// >>> 9 >> PAGE_4BYTES_SHIFT
// 2
// >>> 2 << PAGE_4BYTES_SHIFT
// 8
unsigned long
PAGE_2M_LOWER_ALIGN(unsigned long addr)
{
    return ((addr >> PAGE_2M_SHIFT) << PAGE_2M_SHIFT);
}

// >>> 23 >> 21
// 0
// >>> 1024 * 1024 >> 21
// 0
// >>> 1024 * 1024* 2 + 12 >> 21
// 1
// >>> 1024 * 1024* 2  >> 21
// 1
// >>> 1024 * 1024* 2 + 2 >> 21
// 1
unsigned long
BYTES_NUM_TO_PAGE_2M_NUM(unsigned long bytes_num)
{
    return (bytes_num >> PAGE_2M_SHIFT);
}

// althrough the two functions are logically same,  but their semantic not same.
// page index is from 0 to N
unsigned long
ADDR_TO_PAGE_2M_NORMAL_INDEX(unsigned long addr)
{
    return BYTES_NUM_TO_PAGE_2M_NUM(addr);
}

unsigned long
ADDR_TO_PAGE_2M_64BITS_LOWER_INDEX(unsigned long addr)
{
    // 2^3 * 2^3 = 8 * 8 = 64 bits
    static const unsigned int BITS_64_SHIFT = 6;
    return (ADDR_TO_PAGE_2M_NORMAL_INDEX(addr) >> BITS_64_SHIFT);
}

unsigned long
PAGE_2M_NUM_TO_BYTES_NUM(unsigned long pages_num)
{
    return (pages_num << PAGE_2M_SHIFT);
}

// >>> BYTES_NUM_UPPER_ALIGN_BYTES(12,4)
// 12
// >>> BYTES_NUM_UPPER_ALIGN_BYTES(13,4)
// 16
// >>> BYTES_NUM_UPPER_ALIGN_BYTES(14,4)
// 16
// >>> BYTES_NUM_UPPER_ALIGN_BYTES(15,4)
// 16
// >>> BYTES_NUM_UPPER_ALIGN_BYTES(16,4)
// 16
// def BYTES_NUM_UPPER_ALIGN_BYTES(size, align_bytes):
//     BYTES_SIZE = align_bytes
//     BYTES_MASK = (~(BYTES_SIZE - 1))
//     return ((size + BYTES_SIZE - 1) & BYTES_MASK)
unsigned long
BYTES_NUM_UPPER_ALIGN_BYTES(unsigned long size, unsigned long align_bytes);
unsigned long
BYTES_NUM_UPPER_ALIGN_BYTES(unsigned long size, unsigned long align_bytes)
{
    const unsigned long BYTES_SIZE = align_bytes;
    const unsigned long BYTES_MASK = (~(BYTES_SIZE - 1));
    return ((size + BYTES_SIZE - 1) & BYTES_MASK);
}

// >>> BYTES_NUM_LOWER_ALIGN_BYTES(0, 4 * 32)
// 128
// >>> BYTES_NUM_LOWER_ALIGN_BYTES(1, 4 * 32)
// 128
// >>> BYTES_NUM_LOWER_ALIGN_BYTES(2, 4 * 32)
// 130
// >>> BYTES_NUM_LOWER_ALIGN_BYTES(3, 4 * 32)
// 130
// >>> BYTES_NUM_LOWER_ALIGN_BYTES(4, 4 * 32)
// 128
// >>> BYTES_NUM_LOWER_ALIGN_BYTES(5, 4 * 32)
// 128
// >>> BYTES_NUM_LOWER_ALIGN_BYTES(6, 4 * 32)
// 130
// >>> BYTES_NUM_LOWER_ALIGN_BYTES(7, 4 * 32)
// 130
// >>> BYTES_NUM_LOWER_ALIGN_BYTES(8, 4 * 32)
// 136
// >>> BYTES_NUM_LOWER_ALIGN_BYTES(9, 4 * 32)
// 136
// >>> BYTES_NUM_LOWER_ALIGN_BYTES(10, 4 * 32)
// 138
// >>> BYTES_NUM_LOWER_ALIGN_BYTES(11, 4 * 32)
// 138
// >>> BYTES_NUM_LOWER_ALIGN_BYTES(12, 4 * 32)
// 136
// >>> BYTES_NUM_LOWER_ALIGN_BYTES(13, 4 * 32)
// 136
// >>> BYTES_NUM_LOWER_ALIGN_BYTES(14, 4 * 32)
// 138
// >>> BYTES_NUM_LOWER_ALIGN_BYTES(15, 4 * 32)
// 138
// >>> BYTES_NUM_LOWER_ALIGN_BYTES(16, 4 * 32)
// 144
unsigned long
BYTES_NUM_LOWER_ALIGN_BYTES(unsigned long size, unsigned long align_bytes);
unsigned long
BYTES_NUM_LOWER_ALIGN_BYTES(unsigned long size, unsigned long align_bytes)
{
    const unsigned long BYTES_SIZE = align_bytes;
    const unsigned long BYTES_MASK = (~(BYTES_SIZE - 1));
    return ((size + BYTES_SIZE) & BYTES_MASK);
}

// >>> BYTES_NUM_BITS_UPPER_ALIGN_BYTES(12*8, 4)
// 12
// >>> BYTES_NUM_BITS_UPPER_ALIGN_BYTES(13*8, 4)
// 16
// >>> BYTES_NUM_BITS_UPPER_ALIGN_BYTES(14*8, 4)
// 16
// >>> BYTES_NUM_BITS_UPPER_ALIGN_BYTES(15*8, 4)
// 16
// >>> BYTES_NUM_BITS_UPPER_ALIGN_BYTES(16*8, 4)
// 16
// >>> BYTES_NUM_BITS_UPPER_ALIGN_BYTES(17*8, 4)
// 20
// >>> BYTES_NUM_BITS_UPPER_ALIGN_BYTES(1, 4)
// 4
// >>> BYTES_NUM_BITS_UPPER_ALIGN_BYTES(2, 4)
// 4
// >>> BYTES_NUM_BITS_UPPER_ALIGN_BYTES(6, 4)
// 4
// >>> BYTES_NUM_BITS_UPPER_ALIGN_BYTES(10, 4)
// 4
// >>> BYTES_NUM_BITS_UPPER_ALIGN_BYTES(0, 4)
// 0
// >>> BYTES_NUM_BITS_UPPER_ALIGN_BYTES(4*10, 4)
// 8
// def BYTES_NUM_BITS_UPPER_ALIGN_BYTES(size, align_bytes):
//     BYTES_SIZE = align_bytes
//     BYTES_MASK = (~(BYTES_SIZE - 1))
//     bytes_size = (size + BYTES_SIZE * 8 - 1) / 8
//     return (int(bytes_size) & BYTES_MASK)
unsigned long
BYTES_NUM_BITS_UPPER_ALIGN_BYTES(unsigned long bits_size, unsigned long align_bytes);
unsigned long
BYTES_NUM_BITS_UPPER_ALIGN_BYTES(unsigned long bits_size, unsigned long align_bytes)
{
    const unsigned long BYTES_SIZE = align_bytes;
    const unsigned long BYTES_MASK = (~(BYTES_SIZE - 1));
    const unsigned long bytes_size = (bits_size + BYTES_SIZE * 8 - 1) / 8;
    return (bytes_size & BYTES_MASK);
}

unsigned long
VirtualToPhysicalAddr(unsigned long addr)
{
    return addr - PAGE_OFFSET;
}

unsigned long*
PhysicalToVirtualAddr(unsigned long addr)
{
    return (unsigned long*)(addr + PAGE_OFFSET);
}

// However, since 7E00h is a physical address,
// it must be mapped through the page table before it can be used by the
// program. The converted linear address is 0xffff800000007e00h, which is the
// target address for the program to operate
void init_memory()
{
    unsigned long total_mem = 0;
    struct E820Entry* p = NULL;
    color_printk(
        BLUE,
        BLACK,
        "Display Physics Address MAP,Type(1:RAM,2:ROM or Reserved,3:ACPI Reclaim "
        "Memory,4:ACPI NVS Memory,Others:Undefine)\n");

    p = (struct E820Entry*)0xffff800000007e00;

    for (int i = 0; i < E820_MAX_ENTRIES; ++i)
    {
        color_printk(ORANGE,
                     BLACK,
                     "Address:%#018lx\tLength:%#018lx\tType:%#010x\n",
                     p->address,
                     p->length,
                     p->type);
        // type = 1 is useable physical memory space
        // the usable physical memory space capacity is combined of two parts:
        // part one: capacity is 0x9f000
        // part two: capacity is 0x7fef0000
        // total is 0x7ff8f000 Byte near 2GB  （megs: 2048 see conf folder
        // linux-bochsrc）
        if (p->type == E820_RAM)
        {
            total_mem += p->length;
        }

        global_memory_descriptor.e820.entries[i].address = p->address;
        global_memory_descriptor.e820.entries[i].length = p->length;
        global_memory_descriptor.e820.entries[i].type = p->type;
        global_memory_descriptor.e820.number_entries = i + 1;

        p++;
        // normally, type is cannot greater than 4, if it is, program must meet junk
        // data while running
        if (p->type > E820_ACPI_NVS || p->length == 0 || p->type < E820_RAM)
        {
            color_printk(RED, BLACK, "Junk data in E820 table\n");
            break;
        }
    }

    color_printk(ORANGE, BLACK, "OS Can Used Total RAM:%#018lx\n", total_mem);

    // After detecting the available physical memory segments,
    // the start address of these segments is aligned to the upper boundary of the
    // 2 MB page using the macro function PAGE_2M_ALIGN, and the aligned address
    // is the valid memory start address of the segment. The end address of these
    // segments is obtained by adding the original start address of the segment
    // and the segment length, and then the result of the calculation is aligned
    // by shifting to the lower boundary of the 2 MB page, or by using the
    // previously defined macro constant PAGE_2M_MASK to align the lower boundary
    // of the page. If the calculated start address is less than or equal to the
    // calculated end address, the segment is considered a valid memory segment,
    // and the number of available physical pages is calculated and the total
    // number of available physical pages is printed on the screen.
    unsigned long total_pages = 0;
    for (unsigned long i = 0; i < global_memory_descriptor.e820.number_entries;
         ++i)
    {
        unsigned long start, end;
        // usable physical memory space
        if (global_memory_descriptor.e820.entries[i].type == E820_RAM)
        {
            start = PAGE_2M_UPPER_ALIGN(global_memory_descriptor.e820.entries[i].address);
            end = PAGE_2M_LOWER_ALIGN(global_memory_descriptor.e820.entries[i].address + global_memory_descriptor.e820.entries[i].length);
            if (end > start)
            {
                total_pages += BYTES_NUM_TO_PAGE_2M_NUM(end - start);
            }
        }
    }

    color_printk(ORANGE,
                 BLACK,
                 "OS Can Used Total 2M Pages:%#010x=%010d\n",
                 total_pages,
                 total_pages);

    // |  -----  physical memory space for pages (ROM or void memory pages etc)
    // -----  |   -- isolated space --    | bitmaps (bitmap0, bitmap1,... bitmapN)
    unsigned long end_addr_of_physical_space = global_memory_descriptor.e820
                                                   .entries[global_memory_descriptor.e820.number_entries - 1]
                                                   .address
                                               + global_memory_descriptor.e820
                                                     .entries[global_memory_descriptor.e820.number_entries - 1]
                                                     .length;

    // bits map pointer, pointer to end address of kernel
    // This is done to preserve a small section of isolated space to prevent
    // misuse from corrupting data in other spaces (this is why  upper alginment
    // needed)
    global_memory_descriptor.bits_map = (unsigned long*)PAGE_4K_UPPER_ALIGN(global_memory_descriptor.end_brk);
    // number of avaible 2M pages
    // This physical address space avaible pages that include not only available
    // physical memory, but also memory voids and ROM address space
    global_memory_descriptor.bits_size = BYTES_NUM_TO_PAGE_2M_NUM(end_addr_of_physical_space);

    global_memory_descriptor.bits_length = BYTES_NUM_BITS_UPPER_ALIGN_BYTES(global_memory_descriptor.bits_size, sizeof(long));
    // The entire bits_map space is set all the way to mark non-memory pages
    // (memory voids and ROM space) as used, and then the available physical
    // memory pages in the map bitmap are programmatically reset later.
    memset(global_memory_descriptor.bits_map,
           0xff,
           global_memory_descriptor.bits_length);
    color_printk(ORANGE,
                 BLACK,
                 "bitsmap bytes length : %d\n",
                 global_memory_descriptor.bits_length);

    // | bitmap | pages |
    // Create the storage space and allocation records for the page array. struct
    // page is located after the bitmap, and the number of elements in the array
    // is the number of pages that can be allocated in the physical address space,
    // which is allocated and calculated in a similar way to the bitmap, except
    // that the array is zeroed out for subsequent initialization procedures.
    unsigned long pages_addr = (unsigned long)global_memory_descriptor.bits_map + global_memory_descriptor.bits_length;
    global_memory_descriptor.pages = (struct Page*)PAGE_4K_UPPER_ALIGN(pages_addr);
    global_memory_descriptor.pages_size = BYTES_NUM_TO_PAGE_2M_NUM(end_addr_of_physical_space);
    global_memory_descriptor.pages_length = BYTES_NUM_UPPER_ALIGN_BYTES(
        global_memory_descriptor.pages_size * sizeof(struct Page), sizeof(long));
    memset(global_memory_descriptor.pages,
           0x00,
           global_memory_descriptor.pages_length);

    // | pages | zones |
    // Same as above, however, since the number of elements in the struct zone
    // array cannot be calculated for the time being, the value is assigned to 0
    // for the time being, and the zones_length is calculated according to the 5
    // struct zone structures for the time being
    unsigned long zones_addr = (unsigned long)global_memory_descriptor.pages + global_memory_descriptor.pages_length;
    global_memory_descriptor.zones = (struct Zone*)PAGE_4K_UPPER_ALIGN(zones_addr);
    global_memory_descriptor.zones_size = 0;
    const unsigned long INIT_ZONES_NUMBER = 5;
    global_memory_descriptor.zones_length = BYTES_NUM_UPPER_ALIGN_BYTES(INIT_ZONES_NUMBER * sizeof(struct Zone), sizeof(long));

    for (unsigned long i = 0; i < global_memory_descriptor.e820.number_entries;
         ++i)
    {
        // usable physical memory space
        if (global_memory_descriptor.e820.entries[i].type == E820_RAM)
        {
            unsigned long start = PAGE_2M_UPPER_ALIGN(global_memory_descriptor.e820.entries[i].address);
            unsigned long end = PAGE_2M_LOWER_ALIGN(global_memory_descriptor.e820.entries[i].address + global_memory_descriptor.e820.entries[i].length);
            if (end > start)
            {
                struct Zone* new_zone = global_memory_descriptor.zones + global_memory_descriptor.zones_size;
                global_memory_descriptor.zones_size++;

                new_zone->start_address = start;
                new_zone->end_address = end;
                new_zone->length = end - start;

                new_zone->page_using_count = 0;
                new_zone->page_free_count = BYTES_NUM_TO_PAGE_2M_NUM(new_zone->length);
                new_zone->total_pages_ref_count = 0;

                new_zone->attribute = 0;
                new_zone->gmt = &global_memory_descriptor;

                new_zone->number_pages = BYTES_NUM_TO_PAGE_2M_NUM(new_zone->length);
                new_zone->pages = global_memory_descriptor.pages + BYTES_NUM_TO_PAGE_2M_NUM(start);

                struct Page* page_index = new_zone->pages;
                for (unsigned long j = 0; j < new_zone->number_pages;
                     j++, page_index++)
                {
                    page_index->zone = new_zone;
                    page_index->physical_address = start + PAGE_2M_SIZE * j;
                    page_index->attribute = 0UL;
                    page_index->reference_count = 0UL;
                    page_index->create_time = 0UL;

                    // sizeof(*bitmap) = 8 bytes = 64 bits
                    // *bitmap inital value is : 11111...11111 (64-bit),
                    // 111111....1111(64bit), .... N bitmaps are composed of a array of
                    // bitmap  *(bitmap + N)  = bitmaps[32] Converts the physical address
                    // represented by the current struct page structure to the
                    // corresponding bit in bits_map. Since the bits_map has previously
                    // been set to all bit1, then at this moment the corresponding bit of
                    // the available physical page is set to 0 (because it is an iso-or
                    // operation), marking the corresponding available physical page as
                    // unused
                    mark_page_used_in_bitsmap_clear(global_memory_descriptor.bits_map, page_index->physical_address);
                }
            }
        }
    }

    global_memory_descriptor.pages->zone = global_memory_descriptor.zones;

    global_memory_descriptor.pages->physical_address = 0UL;
    global_memory_descriptor.pages->attribute = 0UL;
    global_memory_descriptor.pages->reference_count = 0UL;
    global_memory_descriptor.pages->create_time = 0UL;

    // After some traversal, all available physical memory pages have been initialized, but since the 0-2MB physical memory page contains multiple physical memory segments,
    // including the kernel program, the page must be specially initialized before the number of elements in the struct zone space structure array can be calculated
    const unsigned long zones_total_size = global_memory_descriptor.zones_size * sizeof(struct Zone);
    global_memory_descriptor.zones_length = BYTES_NUM_UPPER_ALIGN_BYTES(zones_total_size, sizeof(long));

    color_printk(ORANGE, BLACK, "bits_map:%#018lx,bits_size:%#018lx,bits_length:%#018lx\n", global_memory_descriptor.bits_map, global_memory_descriptor.bits_size, global_memory_descriptor.bits_length);
    color_printk(ORANGE, BLACK, "pages_struct:%#018lx,pages_size:%#018lx,pages_length:%#018lx\n", global_memory_descriptor.pages, global_memory_descriptor.pages_size, global_memory_descriptor.pages_length);
    color_printk(ORANGE, BLACK, "zones_struct:%#018lx,zones_size:%#018lx,zones_length:%#018lx\n", global_memory_descriptor.zones, global_memory_descriptor.zones_size, global_memory_descriptor.zones_length);

    ZONE_DMA_INDEX = 0;
    ZONE_NORMAL_INDEX = 0;

    // If the starting address of the current area is 0x100000000, this area index value will be recorded in the global order ZONE_UNMAPPED_INDEX,
    // indicating that the physical address memory page starting from this area space is not mapped through the page table.
    // Finally, adjust the value of the member variable end_of_struct to record the end address of the above structure,
    // and reserve a section of memory space to prevent out-of-bounds access
    for (unsigned long i = 0; i < global_memory_descriptor.zones_size; ++i)
    {
        struct Zone* zone = &global_memory_descriptor.zones[i];

        color_printk(ORANGE, BLACK, "zone_start_address:%#018lx,zone_end_address:%#018lx,zone_length:%#018lx,pages_group:%#018lx,pages_length:%#018lx\n",
                     zone->start_address, zone->end_address, zone->length, zone->pages, zone->number_pages);

        if (zone->start_address == 0x100000000)
        {
            ZONE_UNMAPED_INDEX = i;
        }
    }

    unsigned long end_of_struct_addr = (unsigned long)global_memory_descriptor.zones + global_memory_descriptor.zones_length;
    global_memory_descriptor.end_of_struct = BYTES_NUM_LOWER_ALIGN_BYTES(end_of_struct_addr, sizeof(long) * 32); ////need a blank to separate memory_management_struct

    color_printk(ORANGE, BLACK, "start_code:%#018lx,end_code:%#018lx,end_data:%#018lx,end_brk:%#018lx,end_of_struct:%#018lx\n",
                 global_memory_descriptor.start_kernel_code,
                 global_memory_descriptor.end__kernel_code,
                 global_memory_descriptor.end_data,
                 global_memory_descriptor.end_brk,
                 global_memory_descriptor.end_of_struct);

    // After the memory management unit is initialized, the Page structure of the physical page occupied by
    // the memory management unit (global_memory_descriptor) must also be initialized
    for (unsigned long j = 0; j <= BYTES_NUM_TO_PAGE_2M_NUM(VirtualToPhysicalAddr(global_memory_descriptor.end_of_struct)); ++j)
    {
        init_page(&global_memory_descriptor.pages[j], PAGE_TABLE_MAPPED | KERNEL_INIT | ACTIVE | KERNEL);
    }

    // The remaining code of the init_memory function is used to empty the page table entries,
    // which were used for the consistency page table mapping and no longer need to keep the consistency page table mapping at this moment

    // Get the page directory base address, the base address is stored in the CR3 control register
    global_cr3 = get_gdt();

    // start address of several levels of page tables, the page table initialized by head.S
    color_printk(INDIGO, BLACK, "Global_CR3\t:%#018lx\n", global_cr3);
    color_printk(INDIGO, BLACK, "*Global_CR3\t:%#018lx\n", *PhysicalToVirtualAddr((unsigned long)global_cr3) & (~0xff));
    color_printk(PURPLE, BLACK, "**Global_CR3\t:%#018lx\n", *PhysicalToVirtualAddr(*PhysicalToVirtualAddr((unsigned long)global_cr3) & (~0xff)) & (~0xff));

    // Immediately afterwards,
    // the first 10 page table entries in the page table directory (PML4 page table) are zeroed out in order to eliminate the consistency page table mapping
    // (in fact, it is sufficient to zero out the first entry).
    // Although these page table entries are cleared, they do not take effect immediately and flush_tlb must be used to make the changes take effect
    for (int i = 0; i < 10; ++i)
    {
        *(PhysicalToVirtualAddr((unsigned long)global_cr3) + i) = 0UL;
    }

    flush_tlb();
}

unsigned long init_page(struct Page* page, unsigned long flags)
{
    if (page->attribute == 0)
    {
        mark_page_used_in_bitsmap(global_memory_descriptor.bits_map, page->physical_address);
        page->attribute = flags;
        page->reference_count++;
        page->zone->page_using_count++;
        page->zone->page_free_count--;
        page->zone->total_pages_ref_count++;
    }
    else if (has_atrribute(page->attribute, REFERENCED) || has_atrribute(page->attribute, KERNEL_SHARE_TO_USER)
             || has_atrribute(flags, REFERENCED) || has_atrribute(flags, KERNEL_SHARE_TO_USER))
    {
        page->attribute |= flags;
        page->reference_count++;
        page->zone->total_pages_ref_count++;
    }
    else
    {
        mark_page_used_in_bitsmap(global_memory_descriptor.bits_map, page->physical_address);
        page->attribute |= flags;
    }

    return 0;
}

unsigned long clean_page(struct Page* page)
{
    if (page->attribute == 0)
    {
        page->attribute = 0;
    }
    else if (has_atrribute(page->attribute, REFERENCED) || has_atrribute(page->attribute, KERNEL_SHARE_TO_USER))
    {
        page->reference_count--;
        page->zone->total_pages_ref_count--;

        if (page->reference_count == 0)
        {
            page->attribute = 0;
            page->zone->page_using_count--;
            page->zone->page_free_count++;
        }
    }
    else
    {
        unmark_page_used_in_bitsmap(global_memory_descriptor.bits_map, page->physical_address);
        page->attribute = 0;
        page->reference_count = 0;
        page->zone->page_using_count--;
        page->zone->page_free_count++;
        page->zone->total_pages_ref_count--;
    }

    return 0;
}
