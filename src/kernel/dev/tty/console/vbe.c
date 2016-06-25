/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2016 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * This file is part of Nanvix.
 * 
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 * 
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/hal.h>
#include <dev/vga.h>
#include <sys/types.h>
#include <stdint.h>
#include "../tty.h"
#include "fonts/vcr.h"

/*
 * Console cursor.
 */
PRIVATE struct 
{
	int x; /* Horizontal axis position. */
	int y; /* Vertical axis position.   */
} cursor;

/* Maximum width taking into account the character size. */
#define CHAR_WIDTH ( (SCREEN_WIDTH / VCR_WIDTH) * VCR_WIDTH )

/* Maximum height. */
#define CHAR_HEIGHT ( (SCREEN_HEIGHT / VCR_HEIGHT) * VCR_HEIGHT )

/* Video memory.*/
PRIVATE uint32_t *video = (uint32_t*)VBE_DISPI_LFB_PHYSICAL_ADDRESS;

/*
 * Draws an char on screen.
 * @param x x-axis start of drawing.
 * @param y y-axis start of drawing.
 * @param c Character to be printed.
 * @param color An 32bit color RGB, the last 8 bits arent used.
 */
PRIVATE void draw_char(int x, int y, char c, uint32_t color)
{
	int start;        /* Start point to draw. */
	unsigned offset;  /* Offset in bytes.     */ 

	start = (c-VCR_FAC) * (VCR_HEIGHT*2);
	offset  = VIDEO_OFFSET(x,y);

	for(int i = start; i < start+(VCR_HEIGHT*2); i+=2)
	{
		unsigned short c = ( (VCR_OSD[i] << 8) | VCR_OSD[i+1] ) >> 4;

		for(int j = VCR_WIDTH; j > 0; j--)
		{
			if ( (c & (1 << j)) )
				video[offset] = color;

			offset++;
		}
		offset += SCREEN_WIDTH;
		offset -= VCR_WIDTH;
	}
}

/*
 * Scrolls down the console by one row.
 */
PRIVATE void console_scrolldown(void)
{
    uint32_t *p; /* Video pointer. */
    p = video;
    
    /* Pull lines up. */
    for (p = video; p < (video + (SCREEN_HEIGHT-VCR_HEIGHT)*(SCREEN_WIDTH)); p++)
        *p = *(p + (SCREEN_WIDTH*VCR_HEIGHT));
    
    /* Blank last line. */
    for (int i = 0; i < VCR_WIDTH*VCR_HEIGHT*CHAR_WIDTH; i++, p++)
        *p = 0;

    /* Set cursor position. */
    cursor.x = 0; cursor.y = CHAR_HEIGHT - VCR_HEIGHT;
}

/*
 * Outputs a colored ASCII character on the console device.
 */
PRIVATE void c_put(uint8_t ch, uint32_t color)
{	
	/* Parse character. */
    switch (ch)
    {
        /* New line. */
        case '\n':
            cursor.y += VCR_HEIGHT;
            cursor.x = 0;
            break;
            
        /* Tabulation. */
        case '\t':
            /* FIXME. */
            cursor.x += (4*VCR_WIDTH) - (cursor.x & 3);
            break;
            
        /* Backspace. */
        case '\b':
            if (cursor.x > 0)
                cursor.x -= VCR_WIDTH;
            else if (cursor.y > 0)
            {
                cursor.x = CHAR_WIDTH - VCR_WIDTH;
                cursor.y -= VCR_HEIGHT;
            }
            draw_char(cursor.x,cursor.y,VCR_FAC,BLACK);
            break;

        case ' ':
        	cursor.x += VCR_WIDTH;
        	break;		
        
        /* Any other. */
        default:
            draw_char(cursor.x,cursor.y,ch,color);
            cursor.x += VCR_WIDTH;
            break;
    }

    /* Set cursor position. */
    if (cursor.x >= CHAR_WIDTH)
    {
        cursor.x = 0;
        cursor.y += VCR_HEIGHT;
    }
    if (cursor.y >= CHAR_HEIGHT)
        console_scrolldown();
}

/*
 * Clears the console.
 */
PRIVATE void c_clear(void)
{
	unsigned *p; /* Video pointer.       */
	int n;       /* Bytes to be cleared. */

	/* Blank all lines. */
	p = video;
	n = SCREEN_WIDTH*SCREEN_HEIGHT*4;
	
	while (n > 0)
	{
		*p++ = 0;
		n -= 4;
	}

	/* Set console cursor position. */
	cursor.x = cursor.y = 0;
}

/*
 * Flushes a buffer on the console device.
 */
PRIVATE void c_write(struct kbuffer *buffer)
{
	uint8_t ch; /* Char to be written. */
	
	/* Outputs all characters. */
	while (!KBUFFER_EMPTY((*buffer)))
	{ 
		KBUFFER_GET((*buffer), ch);
	
		c_put(ch, WHITE);
	}
}

/*
 * Console handler interface.
 */
PRIVATE struct console_handler chandler = {
	&c_put,   /* console_put().   */
	&c_clear, /* console_clear(). */
	&c_write  /* console_write(). */
};

/*
 * Initializes the console driver.
 */
PUBLIC void cvbemode_init(void)
{
	/* Register the handler. */
	if ( console_register(&chandler) )
		kpanic("failed to set console handler");
}
