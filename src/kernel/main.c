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

void Start_Kernel(void)
{
	init_screen(1440, 900);
	color_printk(YELLOW,BLACK,"Hello\tMiko\tOS!\n");

	
	test_divid_zero();
	while(true);
}