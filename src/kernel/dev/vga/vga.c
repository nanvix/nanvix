/* 
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2016 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * This file is part of Nanvix.
 * 
 * Nanvix is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix.  If not, see <http://www.gnu.org/licenses/>.
 */

#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <dev/vga.h>
#include <stdint.h>

/**
 * Write to Bochs VBE register.
 * @param index Register.
 * @param data Data to be written.
 */
PRIVATE void write_register(uint16_t index, uint16_t data)
{
	outputw(VBE_DISPI_IOPORT_INDEX, index);
	outputw(VBE_DISPI_IOPORT_DATA, data);
}

/**
 * Read to Bochs VBE register.
 * @param index Register.
 */
PRIVATE unsigned short read_register(uint16_t index)
{
	outputw(VBE_DISPI_IOPORT_INDEX, index);
	return inputw(VBE_DISPI_IOPORT_DATA);
}

/**
 * Checks if available the BGA Version.
 */
PRIVATE int bga_check(void)
{
	return (read_register(VBE_DISPI_INDEX_ID) == VBE_DISPI_ID5);
}

/**
 * Configure video width, height, depth, linear frame buffer and
 * clear memory.
 * @param width Screen width.
 * @param height Screen height.
 * @param depth Color depth.
 * @param lfb Use or not Linear frame buffer.
 * @param clear_mem Clear or not memory video.
 */
PRIVATE void set_video_mode(uint16_t width, uint16_t height,
 uint16_t depth, uint8_t lfb, uint8_t clear_mem)
{
	write_register(VBE_DISPI_INDEX_ENABLE, VBE_DISPI_DISABLED);
	write_register(VBE_DISPI_INDEX_XRES, width);
	write_register(VBE_DISPI_INDEX_YRES, height);
	write_register(VBE_DISPI_INDEX_BPP, depth);
	write_register(VBE_DISPI_INDEX_ENABLE, VBE_DISPI_ENABLED |
		(lfb ? VBE_DISPI_LFB_ENABLED : 0) |
		(clear_mem ? 0 : VBE_DISPI_NOCLEARMEM));
}

/**
 * Initalizes the video mode.
 */
PUBLIC void vga_init()
{
	if (VIDEO_MODE == 1)
	{
		if ( !bga_check() )
			kpanic("No support to BGA Version");

		set_video_mode(SCREEN_WIDTH,SCREEN_HEIGHT,DEPTH,1,1);
	}
}
