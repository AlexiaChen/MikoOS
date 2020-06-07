#include <stdarg.h>
#include <stdint.h>

#include "printk.h"
#include "util.h"
#include "linkage.h"

static int TAB_HOLD_CHARS = 8;
static int ONE_CHAR_WIDTH = 8; // 8 pixel
static int ONE_CHAR_HEIGHT = 16; // 16 pixel
static int64_t FRAME_BUFFER_MAPPED_ADDRESS = 0xffff800000a00000;

// 0~7 bits represent blue
// 8~15 bits represent green
// 16~23 bits represent red
// 24~31 bits reserved
static int ONE_PIXEL_32_BITS_WIDTH = 4; // 4 bytes

static unsigned int* screen_buffer()
{
    return screen_position.frame_buffer_address;
}

static int screen_width()
{
    return screen_position.x_resolution;
}

static int screen_height()
{
    return screen_position.x_resolution;
}

static int char_width()
{
    return screen_position.x_char_size;
}

static int char_height()
{
    return screen_position.y_char_size;
}

static void cursor_next_line()
{
    screen_position.y_position++; // next row
	screen_position.x_position = 0; // column begin
}

static void cursor_prev_column()
{
    screen_position.x_position--; // next column
}

static void cursor_next_column()
{
    screen_position.x_position++; // next column
}

static void move_cursor_position(int x, int y)
{
    screen_position.x_position = x;
    screen_position.y_position = y;
}

static int cursor_x()
{
    return screen_position.x_position;
}

static int cursor_y()
{
    return screen_position.y_position;
}


static bool_t is_cursor_left_out_of_bound()
{
    return cursor_x() < 0 ? true : false;
}

static bool_t is_cursor_upper_out_of_bound()
{
    return cursor_y() < 0 ? true : false;
}

static bool_t is_cursor_right_out_of_bound()
{
    return (cursor_x() >= (screen_width() / char_width())) ? true : false;
}

static bool_t is_cursor_down_out_of_bound()
{
    return (cursor_y() >= (screen_height() / char_height())) ? true : false;
}

static void set_screen_size(int width, int height)
{
    screen_position.x_resolution = width;
	screen_position.y_resolution = height;
}

static void set_char_size(int width, int height)
{
    screen_position.x_char_size = ONE_CHAR_WIDTH;
	screen_position.y_char_size = ONE_CHAR_HEIGHT;
}

static void set_screen_buffer(unsigned int* buffer_address, unsigned long length)
{
    screen_position.frame_buffer_address = buffer_address;
	screen_position.frame_buffer_length = length;
}

void init_screen(int width, int height)
{
    set_screen_size(width, height);
    move_cursor_position(0, 0);
    set_char_size(ONE_CHAR_WIDTH, ONE_CHAR_HEIGHT);
    set_screen_buffer((unsigned int *)FRAME_BUFFER_MAPPED_ADDRESS, screen_width() * screen_height() * ONE_PIXEL_32_BITS_WIDTH);
}

void test_color(int width, int height)
{
    int *addr = (int *)FRAME_BUFFER_MAPPED_ADDRESS;
	int i;

	for (i = 0; i < width*height; i++)
	{
		*addr = RED;	
		addr +=1;	
	}
	
	for (i = 0; i < width*height; i++)
	{
		*addr = GREEN;		
		addr +=1;	
	}
	
	for (i = 0; i < width*height; i++)
	{
		*addr = BLUE;	
		addr +=1;	
	}
	
	for (i = 0; i < width*height; i++)
	{
		*addr = WHITE;	
		addr +=1;	
	}
}

void putchar(unsigned int * frame_buffer,int x_size,int x,int y,unsigned int front_color,unsigned int back_color,unsigned char font)
{
	int row = 0, column = 0;
	unsigned int * addr = NULL;
	unsigned char * fontp = NULL;
	int testval = 0;
	fontp = font_ascii[font];

	for(row = 0; row < ONE_CHAR_HEIGHT; row++)
	{
		addr = frame_buffer + x_size * ( y + row ) + x;
		testval = 0x100;
		for(column = 0; column < ONE_CHAR_WIDTH; column++)		
		{
			testval = testval >> 1;
			if(*fontp & testval)
				*addr = front_color;
			else
				*addr = back_color;
			addr++;
		}
		fontp++;		
	}
}

int skip_atoi(const char **s)
{
	int i=0;

	while (is_digit(**s))
		i = i*10 + *((*s)++) - '0';
	return i;
}

/*
*/

static char * itoa(char * str, long num, int base, int size, int precision,	int type)
{
	char c,sign,tmp[50];
	const char *digits = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
	int i;

	if (type&SMALL) digits = "0123456789abcdefghijklmnopqrstuvwxyz";
	if (type&LEFT) type &= ~ZEROPAD;
	if (base < 2 || base > 36)
		return 0;
	c = (type & ZEROPAD) ? '0' : ' ' ;
	sign = 0;
	if (type&SIGN && num < 0) {
		sign='-';
		num = -num;
	} else
		sign=(type & PLUS) ? '+' : ((type & SPACE) ? ' ' : 0);
	if (sign) size--;
	if (type & SPECIAL)
		if (base == 16) size -= 2;
		else if (base == 8) size--;
	i = 0;
	if (num == 0)
		tmp[i++]='0';
	else while (num!=0)
		tmp[i++]=digits[do_div(num,base)];
	if (i > precision) precision=i;
	size -= precision;
	if (!(type & (ZEROPAD + LEFT)))
		while(size-- > 0)
			*str++ = ' ';
	if (sign)
		*str++ = sign;
	if (type & SPECIAL)
		if (base == 8)
			*str++ = '0';
		else if (base==16) 
		{
			*str++ = '0';
			*str++ = digits[33];
		}
	if (!(type & LEFT))
		while(size-- > 0)
			*str++ = c;

	while(i < precision--)
		*str++ = '0';
	while(i-- > 0)
		*str++ = tmp[i];
	while(size-- > 0)
		*str++ = ' ';
	return str;
}


/*
*/

int vsprintf(char * buf,const char *fmt, va_list args)
{
	char * str,*s;
	int flags;
	int field_width;
	int precision;
	int len,i;

	int qualifier;		/* 'h', 'l', 'L' or 'Z' for integer fields */

	for(str = buf; *fmt; fmt++)
	{

		if(*fmt != '%')
		{
			*str++ = *fmt;
			continue;
		}
		
        flags = 0;
        bool_t isLoop = true;
        // support %- %+ %  %# %0
        while(isLoop)
        {
            fmt++;
            switch(*fmt)
            {
                case '-':
                    flags |= LEFT;	
                    break;
                case '+':
                    flags |= PLUS;	
                    break;
                case ' ':
                    flags |= SPACE;	
                    break;
                case '#':
                    flags |= SPECIAL;	
                    break;
                case '0':
                    flags |= ZEROPAD;	
                    break;
                default:
                    isLoop = false;
                    break;
            }
        }

        /* get field width */

        field_width = -1;
        if(is_digit(*fmt))
            field_width = skip_atoi(&fmt);
        else if(*fmt == '*')
        {
            fmt++;
            field_width = va_arg(args, int);
            if(field_width < 0)
            {
                field_width = -field_width;
                flags |= LEFT;
            }
        }
			
        /* get the precision */

        precision = -1;
        if(*fmt == '.')
        {
            fmt++;
            if(is_digit(*fmt))
                precision = skip_atoi(&fmt);
            else if(*fmt == '*')
            {	
                fmt++;
                precision = va_arg(args, int);
            }
            if(precision < 0)
                precision = 0;
        }
        
        qualifier = -1;
        if(*fmt == 'h' || *fmt == 'l' || *fmt == 'L' || *fmt == 'Z')
        {	
            qualifier = *fmt;
            fmt++;
        }

        // support %d %p %x %X %i %c %s %o %u %n %%     
        switch(*fmt)
        {
            case 'c':

                if(!(flags & LEFT))
                    while(--field_width > 0)
                        *str++ = ' ';
                *str++ = (unsigned char)va_arg(args, int);
                while(--field_width > 0)
                    *str++ = ' ';
                break;

            case 's':
            
                s = va_arg(args,char *);
                if(!s)
                    s = '\0';
                len = strlen(s);
                if(precision < 0)
                    precision = len;
                else if(len > precision)
                    len = precision;
                
                if(!(flags & LEFT))
                    while(len < field_width--)
                        *str++ = ' ';
                for(i = 0;i < len ;i++)
                    *str++ = *s++;
                while(len < field_width--)
                    *str++ = ' ';
                break;

            case 'o':
                
                if(qualifier == 'l')
                    str = itoa(str,va_arg(args,unsigned long),8,field_width,precision,flags);
                else
                    str = itoa(str,va_arg(args,unsigned int),8,field_width,precision,flags);
                break;

            case 'p':

                if(field_width == -1)
                {
                    field_width = 2 * sizeof(void *);
                    flags |= ZEROPAD;
                }

                str = itoa(str,(unsigned long)va_arg(args,void *),16,field_width,precision,flags);
                break;

            case 'x':

                flags |= SMALL;

            case 'X':

                if(qualifier == 'l')
                    str = itoa(str,va_arg(args,unsigned long),16,field_width,precision,flags);
                else
                    str = itoa(str,va_arg(args,unsigned int),16,field_width,precision,flags);
                break;

            case 'd':
            case 'i':

                flags |= SIGN;
            case 'u':

                if(qualifier == 'l')
                    str = itoa(str,va_arg(args,unsigned long),10,field_width,precision,flags);
                else
                    str = itoa(str,va_arg(args,unsigned int),10,field_width,precision,flags);
                break;

            case 'n':
                
                if(qualifier == 'l')
                {
                    long *ip = va_arg(args,long *);
                    *ip = (str - buf);
                }
                else
                {
                    int *ip = va_arg(args,int *);
                    *ip = (str - buf);
                }
                break;

            case '%':
                
                *str++ = '%';
                break;

            default:

                *str++ = '%';	
                if(*fmt)
                    *str++ = *fmt;
                else
                    fmt--;
                break;
        }

	}
	*str = '\0';
	return str - buf;
}



int color_printk(unsigned int front_color,unsigned int back_color,const char * fmt,...)
{
	int len = 0;
	int count = 0;
	int line = 0;
	va_list args;
	va_start(args, fmt);

	len = vsprintf(buffer,fmt, args);

	va_end(args);

	for(count = 0;count < len || line;count++)
	{
		////	add \n \b \t
		if(line > 0)
		{
			count--;
			goto Label_tab;
		}
		if(*offset(buffer, count) == '\n')
		{
			cursor_next_line();
		}
		else if(*offset(buffer, count) == '\b')
		{
			cursor_prev_column();
            // cursor position out of bound
			if(is_cursor_left_out_of_bound())
			{
				int new_x_position = (screen_width() / char_width() - 1) * char_width();
				int new_y_position = cursor_y() - 1;
                move_cursor_position(new_x_position, new_y_position);
				if(is_cursor_upper_out_of_bound())
				{
                    new_y_position = (screen_height() / char_height() - 1) * char_height();
                    move_cursor_position(cursor_x(), new_y_position);
                }
			}	
			putchar(screen_buffer() , screen_width() , cursor_x() * char_width() , cursor_y() * char_height() , front_color , back_color , ' ');	
		}
		else if(*offset(buffer, count) == '\t')
		{
			line = ((cursor_x() + TAB_HOLD_CHARS) & ~(TAB_HOLD_CHARS - 1)) - cursor_x();

Label_tab:
			line--;
			putchar(screen_buffer() , screen_width() , cursor_x() * char_width() , cursor_y() * char_height() , front_color , back_color , ' ');	
			cursor_next_column();
		}
		else
		{
			putchar(screen_buffer() , screen_width() , cursor_x() * char_width() , cursor_y() * char_height() , front_color , back_color , *offset(buffer, count));
			cursor_next_column();
		}


		if(is_cursor_right_out_of_bound())
		{
			cursor_next_line();
		}
		if(is_cursor_down_out_of_bound())
		{
            move_cursor_position(cursor_x(), 0);
		}

	}
	return len;
}