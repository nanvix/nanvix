/*
 * Copyright(C) 2011-2017 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2017-2017 Clement Rouquier <clementrouquier@gmail.com>
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

#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/debug.h>
#include <nanvix/fs.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <sys/types.h>
#include <errno.h>

/* Maximum RAM disk size. */
#if RAMDISK_SIZE > PGTAB_SIZE
	#error "RAMDISK_SIZE > PGTAB_SIZE"
#endif

/* Number of RAM disks. */
#define NR_RAMDISKS 2
#define RAMDISK1_SIZE		0x010000 /**< Init RAM disk 2 size.                */ 

char ramdisk1 [RAMDISK1_SIZE];

/*
 * RAM disks.
 */
PRIVATE struct
{
	addr_t start; /* Start address.   */
	addr_t end;   /* End address.     */
	size_t size;  /* Size (in bytes). */
} ramdisks[NR_RAMDISKS];

/*
 * Writes to a RAM disk device.
 */
PRIVATE ssize_t ramdisk_write(unsigned minor, const char *buf, size_t n, off_t off)
{
	size_t i;     /* Loop index.       */
	addr_t ptr;   /* Write pointer.    */
	size_t count; /* # bytes to write. */
	
	/* Invalid device. */
	if (minor >= NR_RAMDISKS)
		return (-EINVAL);
	
	ptr = ramdisks[minor].start + off;
	
	/* Invalid offset. */
	if (ptr >= ramdisks[minor].end)
		return (-EINVAL);
	
	/* Read as much as possible. */
	if (ptr + n >= ramdisks[minor].end)
		n = ramdisks[minor].end - ptr;
	
	/* Write in bursts. */
	for (i = 0; i < n; /* noop */)
	{
		count = ((n - i) > BLOCK_SIZE) ? BLOCK_SIZE : (n - i);
		
		kmemcpy((void *)ptr, buf, count);
		
		i += count;
		ptr += count;
		
		/* Avoid starvation. */
		if (n > 0)
			yield();
	}
	
	return ((ssize_t)n);
}

/*
 * Reads from a RAM disk device.
 */
PRIVATE ssize_t ramdisk_read(unsigned minor, char *buf, size_t n, off_t off)
{
	size_t i;     /* Loop index.      */
	addr_t ptr;   /* Write pointer.   */
	size_t count; /* # bytes to read. */
	
	/* Invalid device. */
	if (minor >= NR_RAMDISKS)
		return (-EINVAL);
	
	ptr = ramdisks[minor].start + off;
	
	/* Invalid offset. */
	if (ptr >= ramdisks[minor].end)
		return (-EINVAL);
	
	/* Read as much as possible. */
	if (ptr + n >= ramdisks[minor].end)
		n = ramdisks[minor].end - ptr;
	
	/* Read in bursts. */
	for (i = 0; i < n; /* noop */)
	{
		count = ((n - i) > BLOCK_SIZE) ? BLOCK_SIZE : (n - i);
		
		kmemcpy(buf, (void *)ptr, count);
		
		i += count;
		ptr += count;
		
		/* Avoid starvation. */
		if (n > 0)
			yield();
	}
	
	return ((ssize_t)n);
}

/*
 * Reads a block from a RAM disk device.
 */
PRIVATE int ramdisk_readblk(unsigned minor, buffer_t buf)
{	
	addr_t ptr;
	
	ptr = ramdisks[minor].start + (buffer_num(buf) << BLOCK_SIZE_LOG2);
	
	kmemcpy(buffer_data(buf), (void *)ptr, BLOCK_SIZE);
	
	return (0);
}

/*
 * Writes a block to a RAM disk device.
 */
PRIVATE int ramdisk_writeblk(unsigned minor, buffer_t buf)
{	
	addr_t ptr;
	
	ptr = ramdisks[minor].start + (buffer_num(buf) << BLOCK_SIZE_LOG2);
	
	kmemcpy((void *)ptr, buffer_data(buf), BLOCK_SIZE);
	
	buffer_dirty(buf, 0);
	brelse(buf);
	
	return (0);
}

/*
 * RAM disk device driver interface.
 */
PRIVATE const struct bdev ramdisk_driver = {
	&ramdisk_read,     /* read()     */
	&ramdisk_write,    /* write()    */
	&ramdisk_readblk,  /* readblk()  */
	&ramdisk_writeblk  /* writeblk() */
};

/**
 * @brief Used for debugging
 * @details Tests if all RAM disks are correctly registered
 * 
 * @returns Upon successful completion one is returned. Upon failure, a 
 *          zero is returned instead.
 */
PRIVATE int rmdtst_register(void)
{
	int i;
	for (i=0;i<NR_RAMDISKS;i++)
	{
		if (&(ramdisks[i].start) == NULL)
		{
			kprintf(KERN_DEBUG "rmdsk test: register of disk number %d failed", i);
			return 0;
		}
	}
	return 1;
}

/**
 * @brief Used for debugging
 * @details Tests if ramdisk_write and ramdisk_read works correctly
 * 
 * @param buffer Buffer to be written in the RAM disk.
 * @param tstrmd_lenght Number of characters to be written in the RAM disk.
 * 
 * @returns Upon successful completion one is returned. Upon failure, a 
 *          zero is returned instead.
 */
PRIVATE int rmdtst_rw(char *buffer, int tstrmd_lenght)
{
	int char_count, char_count2;
	if ((char_count = ramdisk_write(0, buffer, tstrmd_lenght, 0)) != tstrmd_lenght)
	{
		if (char_count <= 0)
			kprintf(KERN_DEBUG "rmdsk test: ramdisk_write failed: nothing has been written");
		else
			kprintf("rmdsk test: ramdisk_write failed: what has been written is not what it has to be write");

		return 0;
	}

	if ((char_count2 = ramdisk_read(0,buffer,tstrmd_lenght, 0)) != char_count)
	{
		if (char_count2 <= 0)
			kprintf(KERN_DEBUG "rmdsk test: ramdisk_read failed: nothing has been read");
		else
			kprintf(KERN_DEBUG "rmdsk test: ramdisk_read failed: what has been read is not what has to be read");
		return 0;
	}

	return 1;
}

PUBLIC void test_rmd(void)
{
	char buffer[KBUFFER_SIZE]; /* Temporary buffer.        */
	int tstrmd_lenght = 39; /* Size of message to write in the log */
	kstrncpy(buffer, "rmdsk test: test data input in ramdisk\n", tstrmd_lenght);

	if(!rmdtst_register())
	{
		tst_failed();
		return;
	}

	if(!rmdtst_rw(buffer, tstrmd_lenght))
	{
		tst_failed();
		return;
	}
	tst_passed();
	return;
}

/*
 * Initializes the RAM disk device driver.
 */
PUBLIC void ramdisk_init(void)
{
	int err;
	
	kprintf("dev: initializing ramdisk device driver");

	/* ramdisk[0] = INITRD. */
	ramdisks[0].start = INITRD_VIRT;
	ramdisks[0].end = INITRD_VIRT + INITRD_SIZE;
	ramdisks[0].size = INITRD_SIZE;

	ramdisks[1].start = (addr_t) ramdisk1;
	ramdisks[1].end = ((addr_t) ramdisk1)+RAMDISK1_SIZE;
	ramdisks[1].size = RAMDISK1_SIZE;

	
	err = bdev_register(RAMDISK_MAJOR, &ramdisk_driver);
	
	/* Failed to register ramdisk device driver. */
	if (err)
		kpanic("failed to register RAM disk device driver. ");

	dbg_register(test_rmd, "test_rmd");
}