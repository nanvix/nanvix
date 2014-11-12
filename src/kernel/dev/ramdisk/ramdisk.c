/*
 * Copyright(C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * dev/ramdisk/ramdisk.c - RAM disk driver implementation.
 */

#include <i386/i386.h>
#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <sys/types.h>
#include <errno.h>

/* Maximul RAM disk size. */
#if RAMDISK_SIZE > PGTAB_SIZE
	#error "RAMDISK_SIZE > PGTAB_SIZE"
#endif

/* Number of RAM disks. */
#define NR_RAMDISKS 1

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
	addr_t ptr;
	size_t count;
	
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
	while (n > 0)
	{
		count = (n > BLOCK_SIZE) ? BLOCK_SIZE : n;
		
		kmemcpy((void *)ptr, buf, count);
		
		n -= count;
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
	addr_t ptr;
	size_t count;
	
	/* Invalid device. */
	if (minor >= NR_RAMDISKS)
		return (-EINVAL);
	
	ptr = ramdisks[minor].start + off;
	
	/* Invalid offset. */
	if (ptr >= ramdisks[minor].end)
		return (-EINVAL);
	
	/* Write as much as possible. */
	if (ptr + n >= ramdisks[minor].end)
		n = ramdisks[minor].end - ptr;
	
	/* Read in bursts. */
	while (n > 0)
	{
		count = (n > BLOCK_SIZE) ? BLOCK_SIZE : n;
		
		kmemcpy(buf, (void *)ptr, count);
		
		n -= count;
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
PRIVATE int ramdisk_readblk(unsigned minor, struct buffer *buf)
{	
	addr_t ptr;
	
	ptr = ramdisks[minor].start + buf->num*BLOCK_SIZE;
	
	kmemcpy(buf->data, (void *)ptr, BLOCK_SIZE);
	
	return (0);
}

/*
 * Writes a block to a RAM disk device.
 */
PRIVATE int ramdisk_writeblk(unsigned minor, struct buffer *buf)
{	
	addr_t ptr;
	
	/* Write only dirty buffers. */
	if (!(buf->flags & BUFFER_DIRTY))
		return (0);
		
	ptr = ramdisks[minor].start + buf->num*BLOCK_SIZE;
	
	kmemcpy((void *)ptr, buf->data, BLOCK_SIZE);
	
	buf->flags &= ~BUFFER_DIRTY;
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

/*
 * Initializes the RAM disk device driver.
 */
PUBLIC void ramdisk_init(void)
{
	int err;
	
	kprintf("dev: initializing ramdisk device driver");

	/* ramdisk[0] = INITRD. */
	ramdisks[0].start = INITRD_VIRT;
	ramdisks[0].end = INITRD_VIRT + RAMDISK_SIZE;
	ramdisks[0].size = RAMDISK_SIZE;
	
	err = bdev_register(RAMDISK_MAJOR, &ramdisk_driver);
	
	/* Failed to register ramdisk device driver. */
	if (err)
		kpanic("failed to register RAM disk device driver. ");
}
