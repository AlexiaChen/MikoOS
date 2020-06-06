#ifndef UTIL_H
#define UTIL_H

#define NULL ((void*)0);

#define sti() 		__asm__ __volatile__ ("sti	\n\t":::"memory")
#define cli()	 	__asm__ __volatile__ ("cli	\n\t":::"memory")
#define nop() 		__asm__ __volatile__ ("nop	\n\t")
#define io_mfence() 	__asm__ __volatile__ ("mfence	\n\t":::"memory") // memory fence, prevent reorder

#define false 0
#define true 1

typedef int bool_t;

bool_t is_digit(char c)
{
    return (c >= '0' && c <= '9') ? true : false;
}


unsigned char* offset(unsigned char* buf, int count)
{
    return buf + count;
}

// num / base 
int do_div(int num, int base)
{
    int __res;
    // num = rdx:rax %%rcx is force use 64-bit register rcx not ecx
    __asm__("divq %%rcx":"=a" (num),"=d" (__res):"0" (num),"1" (0),"c" (base));
    return __res; 
}

int strlen(char* str)
{
    register int __res;
    __asm__	 __volatile__  (	
                    "cld	\n\t"
					"repne	\n\t"
					"scasb	\n\t"
					"notl	%0	\n\t"
					"decl	%0	\n\t"
					:"=c"(__res)
					:"D"(str),"a"(0),"0"(0xffffffff)
					:
				);
    return __res;
}

void test_color()
{
    int *addr = (int *)0xffff800000a00000;
	int i;

	for (i = 0; i< 1440*20; i++)
	{
		*addr = 0x00ff0000;	
		addr +=1;	
	}
	
	for (i = 0; i < 1440*20; i++)
	{
		*addr = 0x0000ff00;		
		addr +=1;	
	}
	
	for (i = 0; i < 1440*20; i++)
	{
		*addr = 0x000000ff;	
		addr +=1;	
	}
	
	for (i = 0; i < 1440*20; i++)
	{
		*addr = 0x00ffffff;	
		addr +=1;	
	}
}

#endif