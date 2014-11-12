/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

#include <drivers/ata.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/fs.h>
#include <nanvix/hal.h>
#include <nanvix/int.h>
#include <nanvix/klib.h>
#include <nanvix/pm.h>
#include <sys/types.h>
#include <stdint.h>

/* Buses. */
#define PRI_BUS 0 /* Primary.   */
#define SEC_BUS 1 /* Secondary. */

/* Devices. */
#define MASTER_DEV 0 /* Master. */
#define SLAVE_DEV  1 /* Slave.  */

/* ATA device maximum queue size. */
#define ATADEV_QUEUE_SIZE 64

/* ATA device flags. */
#define ATADEV_VALID   (1 << 0) /* Valid device?     */
#define ATADEV_DISCARD (1 << 1) /* Discard next IRQ? */

/**
 * @brief Block device operation.
 */
struct block_op
{
	char write;         /* Write operation? */
	struct buffer *buf; /* Buffer to use.   */
}; 

/**
 * @brief ATA devices.
 */
PRIVATE struct atadev
{
	/* General information. */
	int flags;             /* Flags (see above).                         */
	struct ata_info info;  /* Device information.                        */
	struct process *chain; /* Process waiting for operation to complete. */
	
	/* Block operation queue. */
	struct
	{
		int size;                                  /* Current size.          */
		int head;                                  /* Head.                  */
		int tail;                                  /* Tail.                  */
		struct block_op blocks[ATADEV_QUEUE_SIZE]; /* Blocks.                */
		struct process *chain;                     /* Processes wanting for  *
		                                            * a slot in the queue.   */
	} queue;
} ata_devices[4];


/**
 * @brief Default I/O ports for ATA controller.
 */
PRIVATE uint16_t pio_ports[2][9] = {
	/* Primary Bus */
	{ 0x1F0, 0x1F1, 0x1F2, 0x1F3, 0x1F4, 0x1F5, 0x1F6, 0x1F7, 0x3F6 },
	/* Secondary Bus */
	{ 0x170, 0x171, 0x172, 0x173, 0x174, 0x175, 0x176, 0x177, 0x376 }
};

/**
 * @brief Forces an 400 ns delay.
 */
PRIVATE void ata_delay(void)
{
	int i;
	
	/* Waste some time. */
	for (i = 0; i < 5; i++)
		iowait();
}

/**
 * @brief          Selects active device on the ATA bus.
 * @param atadevid ATA device ID.
 */
PRIVATE void ata_device_select(int atadevid)
{
	int bus;
	
	bus = ATA_BUS(atadevid);
	
	/* Parse ATA device ID. */
	switch (atadevid)
	{
		/* Primary master. */
		case ATA_PRI_MASTER:
			outputb(pio_ports[bus][ATA_REG_DEVCTL], 0xa0);
			break;
		
		/* Primary slave. */
		case ATA_PRI_SLAVE:
			outputb(pio_ports[bus][ATA_REG_DEVCTL], 0xb0);
			break;
		
		/* Secondary master. */
		case ATA_SEC_MASTER:
			outputb(pio_ports[bus][ATA_REG_DEVCTL], 0xa0);
			break;
		
		/* Secondary slave. */
		case ATA_SEC_SLAVE:
			outputb(pio_ports[bus][ATA_REG_DEVCTL], 0xb0);
			break;
	}
	
	ata_delay();
}

/**
 * @brief      Waits ATA bus to be ready.
 * @param bus  Bus to wait (primary or secondary).
 */
PRIVATE void ata_bus_wait(int bus)
{
	while (inputb(pio_ports[bus][ATA_REG_ASTATUS] & ATA_BUSY))
		/* noop*/ ;
}

/**
 * @brief          Sets up PATA device.
 * @param atadevid ATA device ID.
 * @returns        On success, zero is returned. On failure, a negative number
 *                 is returned instead.
 */
PRIVATE int pata_setup(int atadevid)
{
	int i;                    /* Loop index.             */
	int bus;                  /* Bus number.             */
	uint16_t status;          /* Status register.        */
	struct atadev *dev;       /* ATA device.             */
	struct ata_info *devinfo; /* ATA device information. */
	
	bus = ATA_BUS(atadevid);
	dev = &ata_devices[atadevid];
	devinfo = &dev->info;
	
	/* Probe device. */
	while (1)
	{
		status = inputb(pio_ports[bus][ATA_REG_ASTATUS]);
				
		/* Device is online. */
		if (status & ATA_DRQ)
			break;
					
		/* Error when probing. */
		if (status & ATA_ERR)
			return (-1);
	}
			
	/* Ready information. */
	for(i = 0; i < 256; i++)
	{
		ata_bus_wait(bus);
		devinfo->rawinfo[i] = inputw(pio_ports[bus][ATA_REG_DATA]);
	}
	
	/* Not a ATA device. */
	if (!ata_info_is_ata(devinfo->rawinfo))
		return (-1);
	
	/* Does not support LBA. */
	if (!ata_info_supports_lba(devinfo->rawinfo))
		return (-1);
	
	/* Setup ATA device structure. */
	devinfo->type = ATADEV_PATA;
	devinfo->flags = ATADEV_PRESENT | ATADEV_LBA;
	devinfo->nsectors = devinfo->rawinfo[ATA_INFO_LBA_CAPACITY_1]
					 | devinfo->rawinfo[ATA_INFO_LBA_CAPACITY_2] << 16;
	
	/* Supports DMA. */
	if (ata_info_supports_dma(devinfo->rawinfo))
		devinfo->flags |= ATADEV_DMA;
	
	dev->flags = ATADEV_VALID | ATADEV_DISCARD;
	dev->queue.chain = NULL;
	dev->queue.size = 0;
	dev->queue.head = 0;
	dev->queue.tail = 0;
	dev->queue.chain = NULL;
	
	return (0);
}

/**
 * @brief          Attempts to identify the type of the active ATA device by
 *                 issuing an IDENTIFY command.
 * @param atadevid ATA device ID.
 * @returns        ATA device type. Moreover, if ATA device is of type PATA,
 *                 device identification information is ready to be read.
 */
PRIVATE int ata_identify(int atadevid)
{
	int bus;              /* Bus number.       */
	uint8_t signature[2]; /* Device signature. */
	
	bus = ATA_BUS(atadevid);
	
	/*
	 * ATA specification says that we should set these
	 * values to zero before issuing an IDENTIFY command,
	 * so later we can properly detect ATA and ATAPI 
	 * devices.
	 */
	outputb(pio_ports[bus][ATA_REG_NSECT], 0);
	outputb(pio_ports[bus][ATA_REG_LBAL], 0);
	outputb(pio_ports[bus][ATA_REG_LBAM], 0);
	outputb(pio_ports[bus][ATA_REG_LBAH], 0);
	
	/* identify command. */	
	outputb(pio_ports[bus][ATA_REG_CMD], ATA_CMD_IDENTIFY);
		
	/* No device attached. */
	if (inputb(pio_ports[bus][ATA_REG_ASTATUS]) == 0)
		return (ATADEV_NULL);
	
	ata_bus_wait(bus);
		
	/* Get device signature */
	signature[0] = inputb(pio_ports[bus][ATA_REG_LBAM]);
	signature[1] = inputb(pio_ports[bus][ATA_REG_LBAH]);
	
	/* ATAPI device.  */
	if ((signature[0] == 0x14) && (signature[1] == 0xeb))
		return (ATADEV_PATAPI);
			
	/* SATAPI device. */
	else if ((signature[0] == 0x69) && (signature[1] == 0x96))
		return (ATADEV_SATAPI);
			
	/* SATA device. */
	else if ((signature[0] == 0x3c) && (signature[1] == 0xc3))
		return (ATADEV_SATA);

	/* PATA device. */
	else if ((signature[0] == 0) && (signature[1] == 0))
		return (ATADEV_PATA);

	/* Unknown ATA device. */
	else
		return (ATADEV_UNKNOWN);
}

/**
 * @brief Issues a read operation.
 * 
 * @param atadevid ATA device ID.
 * @param addr     LBA 48-bit address.
 */
PRIVATE void ata_read_op(unsigned atadevid, uint64_t addr)
{
	int bus;     /* Bus number.        */
	byte_t byte; /* Byte used for I/O. */
	
	ata_device_select(atadevid);
	bus = ATA_BUS(atadevid);

	addr <<= 1;

	/*
	 * Set LBA bit, to specify
	 * that the address is in LBA.
	 */
	outputb(pio_ports[bus][ATA_REG_DEVCTL], 0x40);
	
	/* Send the three highest bytes of the address. */
	outputb(pio_ports[bus][ATA_REG_NSECT], 0x00);
	outputb(pio_ports[bus][ATA_REG_LBAL], (addr >> 0x18) & 0xff);
	outputb(pio_ports[bus][ATA_REG_LBAM], (addr >> 0x20) & 0xff);
	outputb(pio_ports[bus][ATA_REG_LBAH], (addr >> 0x28) & 0xff);

	/* Send the three lowest bytes of the address. */
	outputb(pio_ports[bus][ATA_REG_NSECT], 2);
	outputb(pio_ports[bus][ATA_REG_LBAL], (addr >> 0x00) & 0xff);
	outputb(pio_ports[bus][ATA_REG_LBAM], (addr >> 0x08) & 0xff);
	outputb(pio_ports[bus][ATA_REG_LBAH], (addr >> 0x10) & 0xff);

	outputb(pio_ports[bus][ATA_REG_CMD], ATA_CMD_READ_SECTORS_EXT);
	ata_bus_wait(bus);

	/* Query return value. */
	byte = inputb(pio_ports[bus][ATA_REG_ASTATUS]);
	if (byte & ATA_DF)
		kprintf("ATA: device error");
}

/**
 * @brief Issues a write operation.
 * 
 * @param atadevid ATA device ID.
 * @param buf      Block buffer to use.
 */
PRIVATE void ata_write_op(unsigned atadevid, struct buffer *buf)
{
	int bus;       /* Bus number.         */
	size_t i;      /* Loop index.         */
	byte_t byte;   /* Byte used for I/O.  */
	uint64_t addr; /* LBA 48-bit address. */
	word_t word;   /* Word used for I/O.  */
	
	ata_device_select(atadevid);
	bus = ATA_BUS(atadevid);

	addr = buf->num << 1;

	/*
	 * Set LBA bit, to specify
	 * that the address is in LBA.
	 */
	outputb(pio_ports[bus][ATA_REG_DEVCTL], 0x40);
	
	/* Send the three highest bytes of the address. */
	outputb(pio_ports[bus][ATA_REG_NSECT], 0x00);
	outputb(pio_ports[bus][ATA_REG_LBAL], (addr >> 0x18) & 0xff);
	outputb(pio_ports[bus][ATA_REG_LBAM], (addr >> 0x20) & 0xff);
	outputb(pio_ports[bus][ATA_REG_LBAH], (addr >> 0x28) & 0xff);

	/* Send the three lowest bytes of the address. */
	outputb(pio_ports[bus][ATA_REG_NSECT], 2);
	outputb(pio_ports[bus][ATA_REG_LBAL], (addr >> 0x00) & 0xff);
	outputb(pio_ports[bus][ATA_REG_LBAM], (addr >> 0x08) & 0xff);
	outputb(pio_ports[bus][ATA_REG_LBAH], (addr >> 0x10) & 0xff);

	outputb(pio_ports[bus][ATA_REG_CMD], ATA_CMD_WRITE_SECTORS_EXT);
	ata_bus_wait(bus);

	/* Query return value. */
	byte = inputb(pio_ports[bus][ATA_REG_ASTATUS]);
	if (byte & ATA_DF)
	{
		kprintf("ATA: device error");
		return;
	}			
		
	/* Write block. */
	for (i = 0; i < BLOCK_SIZE; i += 2)
	{
		ata_bus_wait(bus);
		word  = (((unsigned char *)(buf->data))[i]);
		word |= (((unsigned char *)(buf->data))[i + 1] << 8);
		outputw(pio_ports[bus][ATA_REG_DATA], word);
		iowait();
	}
	
	/*
	 * Flushes ATA cache. Note that this will
	 * generate a IRQ, which shall be discarded
	 */
	outputb(pio_ports[bus][ATA_REG_CMD], ATA_CMD_FLUSH_CACHE_EXT);
	ata_bus_wait(bus);
	iowait();
	
	buf->flags &= ~(BUFFER_DIRTY | BUFFER_BUSY);
	brelse(buf);
}

/**
 * @brief Sleeps if block operation queue is full.
 */
PRIVATE void sleep_full(struct atadev *dev)
{
	while (dev->queue.size == ATADEV_QUEUE_SIZE)
		sleep(&dev->queue.chain, PRIO_IO);
}

/**
 * @brief Schedules a block disk IO operation.
 * 
 * @param atadevid ATA device ID.
 * @param buf      Block buffer to use.
 * @param write    Write operation?
 */
PRIVATE void ata_sched(unsigned atadevid, struct buffer *buf, int write)
{
	uint64_t addr;      /* LBA 48-bit address. */
	struct atadev *dev; /* ATA device.         */
	
	dev = &ata_devices[atadevid];

	disable_interrupts();
	
		sleep_full(dev);
		
		/*
		 * Mark buffer as busy.
		 * When the read/write operation completes
		 * this flag will be cleared.
		 */
		buf->flags |= BUFFER_BUSY;
		
		/* Insert block on the queue. */
		dev->queue.blocks[dev->queue.tail].buf = buf;
		dev->queue.blocks[dev->queue.tail].write = write;
		dev->queue.tail = (dev->queue.tail + 1)%ATADEV_QUEUE_SIZE;
		dev->queue.size++;
		
		/*
		 * The queue was empty, therefore,
		 * we can process this block right now.
		 */
		if (dev->queue.size == 1)
		{
			addr = dev->queue.blocks[dev->queue.head].buf->num;
			
			if (write)
				ata_write_op(atadevid, buf);
			else
				ata_read_op(atadevid, addr);
		}
	
	enable_interrupts();
}

/**
 * @brief Reads a block from a ATA device.
 * 
 * @param minor Minor device number.
 * @param buf   Block buffer to use.
 * 
 * @returns Zero if successful; non-zero otherwise.
 */
PRIVATE int ata_readblk(unsigned minor, struct buffer *buf)
{
	struct atadev *dev;
	
	/* Invalid minor device. */
	if (minor >= 4)
		return (-EINVAL);
	
	dev = &ata_devices[minor];
	
	/* Device not valid. */
	if (!(dev->flags & ATADEV_VALID))
		return (-EINVAL);
	
	ata_sched(minor, buf, 0);

	/*
	 * Check if the read operation has already
	 * completed. If not, we need to sleep.
	 */
	disable_interrupts();
	if (buf->flags & BUFFER_BUSY)
		sleep(&dev->chain, PRIO_IO);
	enable_interrupts();
	
	return (0);
}

/**
 * @brief Writes a block to a ATA device.
 * 
 * @param minor Minor device number.
 * @param buf   Block buffer to use.
 * 
 * @returns Zero if successful; non-zero otherwise.
 */
PRIVATE int ata_writeblk(unsigned minor, struct buffer *buf)
{
	struct atadev *dev;
	
	/* Invalid minor device. */
	if (minor >= 4)
		return (-EINVAL);
	
	dev = &ata_devices[minor];
	
	/* Device not valid. */
	if (!(dev->flags & ATADEV_VALID))
		return (-EINVAL);
	
	ata_sched(minor, buf, 1);
	
	return (0);
}

/**
 * @brief Reads from a ATA device.
 */
PRIVATE ssize_t ata_read(unsigned minor, char *buf, size_t n, off_t off)
{
	size_t i;              /* Loop index.   */
	block_t blknum;        /* Block number. */
	struct atadev *dev;    /* ATA device.   */
	struct buffer *blkbuf; /* Block buffer. */
	
	/* Invalid minor device. */
	if (minor >= 4)
		return (-EINVAL);
	
	dev = &ata_devices[minor];
	
	/* Device not valid. */
	if (!(dev->flags & ATADEV_VALID))
		return (-EINVAL);
	
	blknum = ALIGN((off & 0xffff), BLOCK_SIZE);
	n = ALIGN(n, BLOCK_SIZE);
	
	if ((blknum*BLOCK_SIZE + n)/ATA_SECTOR_SIZE > dev->info.nsectors)
	
	/* Read blocks. */
	for (i = 0; i < n; i += BLOCK_SIZE)
	{
		blkbuf = bread(DEVID(ATA_MAJOR, minor, BLKDEV), blknum++);
		kmemcpy(buf, blkbuf->data, BLOCK_SIZE);
		brelse(blkbuf);
	}
	
	return ((ssize_t)i);
}

/**
 * @brief ATA device operations.
 */
PRIVATE const struct bdev ata_ops = {
	&ata_read,    /* read()     */
	NULL,         /* write()    */
	&ata_readblk, /* readblk()  */
	&ata_writeblk /* writeblk() */
};

/**
 * @brief Generic ATA interrupt handler.
 * 
 * @param atadevid ATA device ID.
 */
PRIVATE void ata_handler(int atadevid)
{
	int bus;                /* Bus number.         */
	size_t i;               /* Loop index.         */
	struct atadev *dev;     /* ATA device.         */
	struct block_op *blkop; /* Block operation.    */
	struct buffer *buf;     /* Buffer to be used.  */
	word_t word;
	
	bus = ATA_BUS(atadevid);
	dev = &ata_devices[atadevid];
	
	/*
	 * That's weird! A non valid device
	 * is firing an IRQ. Let's just ignore it so.
	 */
	if (!(dev->flags & ATADEV_VALID))
	{
		kprintf("ATA: non valid device %d fired an IRQ", atadevid);
		return;
	}
		
	/* We don't need to handle this IRQ. */
	if (dev->flags & ATADEV_DISCARD)
	{
		dev->flags &= ~ATADEV_DISCARD;
		return;
	}
	
	/* Broken block operation queue. */
	if (dev->queue.size == 0)
	{
		kpanic("ATA: broken block operation queue?");
		goto out;
	}
	
	/* Get first block operation. */
	blkop = &dev->queue.blocks[dev->queue.head];
	dev->queue.head = (dev->queue.head + 1)%ATADEV_QUEUE_SIZE;
	dev->queue.size--;
	
	buf = blkop->buf;
	
	/* Write operation. */
	if (blkop->write)
	{
		/*
		 * Write is done, so 
		 * just ignore next IRQ.
		 */
		ata_bus_wait(bus);
		dev->flags &= ~ATADEV_DISCARD;
	}
	
	/* Read operation. */
	else
	{		
		/* Read block. */
		for (i = 0; i < BLOCK_SIZE; i += 2)
		{
			ata_bus_wait(bus);
			word = inputw(pio_ports[bus][ATA_REG_DATA]);
			((char *)(buf->data))[i] = word & 0xff;
			((char *)(buf->data))[i + 1] = (word >> 8) & 0xff;
		}
	}
	
	/* Process next operation. */
	if (dev->queue.size > 0)
	{
		blkop = &dev->queue.blocks[dev->queue.head];
		
		if (blkop->write)
			ata_write_op(atadevid, blkop->buf);
		else
			ata_read_op(atadevid, blkop->buf->num);
	}

out:

	buf->flags &= ~BUFFER_BUSY;

	/*
	 * Wakeup the process that was waiting for this
	 * operation and the processes that were waiting
	 * for an empty slot in the block operation queue.
	 */
	wakeup(&dev->queue.chain);
	wakeup(&dev->chain);
}

/**
 * @brief Primary ATA interrupt handler.
 */
PRIVATE void ata1_handler(void)
{
	ata_handler(0);
}

/**
 * @brief Secondary ATA interrupt handler.
 */
PRIVATE void ata2_handler(void)
{
	ata_handler(1);
}

/**
 * @brief Initializes the generic ATA driver.
 */
PUBLIC void ata_init(void)
{
	int i;     /* Loop index.    */
	char dvrl; /* Device letter. */
	
	/* Detect devices. */
	for (i = 0, dvrl = 'a'; i < 4; i++, dvrl++)
	{
		kmemset(&ata_devices[i], 0, sizeof(struct atadev));
		
		ata_device_select(i);
		
		/* Setup ATA device. */
		switch (ata_identify(i))
		{
			/* ATAPI.  */
			case ATADEV_PATAPI:
				kprintf("hd%c: ATAPI CD/DVD detected.", dvrl);
				break;
				
			/* SATAPI. */
			case ATADEV_SATAPI:
				kprintf("hd%c: SATAPI CD/DVD detected.", dvrl);
				break;
					
			/* SATA. */
			case ATADEV_SATA:
				kprintf("hd%c: SATA HDD detected.", dvrl);
				break;
			
			/* PATA. */
			case ATADEV_PATA:
				if (pata_setup(i))
					kprintf("hd%c: device not found.", dvrl);
				else
				{
					kprintf("hd%c: PATA HDD detected.", dvrl);
					kprintf("hd%c: %d sectors.", dvrl, 
												ata_devices[i].info.nsectors);
				}
				break;

			/* UNKNOWN. */
			case ATADEV_UNKNOWN:
				kprintf("hd?: unknown ATA device.");
				break;
		}
	}
	
	/* Register interrupt handler. */
	if (set_hwint(INT_ATA1, &ata1_handler))
		kpanic("INT_ATA1 busy");
	if (set_hwint(INT_ATA2, &ata2_handler))
		kpanic("INT_ATA2 busy");
	
	bdev_register(ATA_MAJOR, &ata_ops);
}
