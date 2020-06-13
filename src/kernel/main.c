#include "printk.h"
#include "util.h"

void Start_Kernel(void)
{
	init_screen(1440, 900);
	color_printk(YELLOW,BLACK,"Hello\tMiko\tOS!\n");

	// for test divided by zero fault
	int i;
	i = 1/0;

	while(true);
}