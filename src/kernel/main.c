#include "printk.h"
#include "util.h"

void Start_Kernel(void)
{
	screen_position.x_resolution = 1440;
	screen_position.y_resolution = 900;

	screen_position.x_position = 0;
	screen_position.y_position = 0;

	screen_position.x_char_size = 8;
	screen_position.y_char_size = 16;

	screen_position.frame_buffer_address = (int *)0xffff800000a00000;
	screen_position.frame_buffer_length = (screen_position.x_resolution * screen_position.y_resolution * 4);

	//test_color();

	color_printk(YELLOW,BLACK,"Hello\tMiko\tOS!\n");
	while(1);
}