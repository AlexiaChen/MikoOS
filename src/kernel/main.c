#include "printk.h"
#include "util.h"
#include "gate.h"
#include "trap.h"

// for test divided by zero fault
static void test_divid_zero()
{
	int i;
	i = 1/0;
}

static void test_page_fault()
{
	int i;
	i = *(int*)0xffff80000aa00000;
}

void Start_Kernel(void)
{
	init_screen(1440, 900);
	color_printk(YELLOW,BLACK,"Hello\tMiko\tOS!\n");

	load_TR(8);
	set_tss64(0xffff800000007c00, 0xffff800000007c00, 0xffff800000007c00, 0xffff800000007c00, 0xffff800000007c00, 
		0xffff800000007c00, 0xffff800000007c00, 0xffff800000007c00, 0xffff800000007c00, 0xffff800000007c00);
	sys_vector_handler_init();

	
	//test_divid_zero();
	//test_page_fault();


	while(true);
}
