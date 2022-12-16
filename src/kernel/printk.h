#ifndef __PRINTK_H__
#define __PRINTK_H__

#include "font.h"
#include "linkage.h"
#include <stdarg.h>

#define ZEROPAD 1  /* pad with zero */
#define SIGN 2     /* unsigned/signed long */
#define PLUS 4     /* show plus */
#define SPACE 8    /* space if plus */
#define LEFT 16    /* left justified */
#define SPECIAL 32 /* 0x */
#define SMALL 64   /* use 'abcdef' instead of 'ABCDEF' */

// one pixel 32 bits color width
#define WHITE 0x00ffffff
#define BLACK 0x00000000
#define RED 0x00ff0000
#define ORANGE 0x00ff8000
#define YELLOW 0x00ffff00
#define GREEN 0x0000ff00  //绿
#define BLUE 0x000000ff   //蓝
#define INDIGO 0x0000ffff //靛
#define PURPLE 0x008000ff //紫

/*
 */

extern unsigned char font_ascii[256][16];

char buffer[1024 * 4] = { 0 };

// screen current information
struct position
{
  // screen resolution
  int x_resolution;
  int y_resolution;

  // current cursor postioon (origin point is left-upper corner)
  int x_position;
  int y_position;

  // one char x*y size
  int x_char_size;
  int y_char_size;

  // frame buffer linear address and length
  unsigned int* frame_buffer_address;
  unsigned long frame_buffer_length;
} screen_position;

void
init_screen(int width, int height);

void
test_color(int width, int height);

void
putchar(unsigned int* frame_buffer,
        int x_size,
        int x,
        int y,
        unsigned int front_color,
        unsigned int back_color,
        unsigned char font);

int
skip_atoi(const char** s);

int
vsprintf(char* buf, const char* fmt, va_list args);

int
color_printk(unsigned int front_color,
             unsigned int back_color,
             const char* fmt,
             ...);

#endif
