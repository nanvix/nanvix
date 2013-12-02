/*
 * Copyright(C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * dev/ramdisk/ramdisk.c - RAM disk driver implementation.
 */

#include <i368/i386.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/klib.h>
#include <sys/types.h>

/*
 * RAM disks.
 */
PRIVATE struct
{
	addr_t start; /* Start address.   */
	addr_t end;   /* End address.     */
	size_t size;  /* Size (in bytes). */
} ramdisks[1];

/*
 * Writes to a RAM disk device.
 */
PRIVATE ssize_t ramdisk_write(unsigned minor, const char *buf, size_t n, off_t off)
{
	addr_t ptr;
	
	/* Invalid device. */
	if (minor >= 1)
		return (-EINVAL);
	
	ptr = ramdisks[minor].start + off;
	
	/* Invalid offset. */
	if (ptr >= ramdisks[minor].end)
		return (-EINVAL);
	
	/* Read as much as possible. */
	if (ptr + n >= ramdisks[minor].end)
		n = ramdisks[minor].end - ptr;
	
	kmemcpy(buf, (void *)ptr, n);
	
	return ((ssize_t)n);
}

/*
 * Reads from a RAM disk device.
 */
PRIVATE ssize_t ramdisk_read(unsigned minor, char *buf, size_t n, off_t off)
{
	addr_t ptr;
	
	/* Invalid device. */
	if (minor >= 1)
		return (-EINVAL);
	
	ptr = ramdisks[minor].start + off;
	
	/* Invalid offset. */
	if (ptr >= ramdisks[minor].end)
		return (-EINVAL);
	
	/* Write as much as possible. */
	if (ptr + n >= ramdisks[minor].end)
		n = ramdisks[minor].end - ptr;
	
	kmemcpy(buf, (void *)ptr, n);
	
	return ((ssize_t)n);
}

/*
 * RAM disk device driver interface.
 */
PRIVATE const struct bdev ramdisk_driver = {
	&ramdisk_read, /* read()  */
	&ramdisk_write /* write() */
}

/*
 * Initializes the RAM disk device driver.
 */
PUBLIC void ramdisk_init(void)
{
	int err;

	/* ramdisk[0] = INITRD. */
	ramdisks[0].start = INITRD_START;
	ramdisks[0].end = INITRD_END;
	ramdisks[0].size = INITRD_START - INITRD_END;
	
	err = bdev_register(RAMDISK_MAJOR, &ramdisk_driver);
	
	/* Failed to register ramdisk device driver. */
	if (err)
		kpanic("failed to register RAM disk device driver. ");
	
	kprintf("INIT: RAM disks initialized");
}
