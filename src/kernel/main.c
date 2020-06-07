#include "printk.h"
#include "util.h"

void Start_Kernel(void)
{
	init_screen(1440, 900);
	color_printk(YELLOW,BLACK,"Hello\tMiko\tOS!\n");
	while(true);
}