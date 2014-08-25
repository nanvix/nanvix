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

#include <asm/io.h>
#include <drivers/ata.h>
#include <nanvix/const.h>
#include <nanvix/klib.h>

/* Buses. */
#define PRI_BUS   0 /* Primary.   */
#define SEC_BUS 1 /* Secondary. */

/* Devices. */
#define MASTER_DEV 0 /* Master. */
#define SLAVE_DEV  1 /* Slave.  */

/**
 * @brief ATA devices information.
 */
PRIVATE struct atadev ata_devices[4];

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
	iowait();
	iowait();
	iowait();
	iowait();
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
	int i;              /* Loop index.      */
	int bus;            /* Bus number.      */
	struct atadev *dev; /* ATA device.      */
	uint16_t status;    /* Status register. */
	
	bus = ATA_BUS(atadevid);
	dev = &ata_devices[atadevid];
	
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
		dev->rawinfo[i] = inputw(pio_ports[bus][ATA_REG_DATA]);
	}
	
	/* Not a ATA device. */
	if (!ata_info_is_ata(dev->rawinfo))
		return (-1);
	
	/* Does not support LBA. */
	if (!ata_info_supports_lba(dev->rawinfo))
		return (-1);
	
	/* Setup ATA device structure. */
	dev->type = ATADEV_PATA;
	dev->flags = ATADEV_PRESENT | ATADEV_LBA;
	dev->nsectors = dev->rawinfo[ATA_INFO_LBA_CAPACITY_1]
					 | dev->rawinfo[ATA_INFO_LBA_CAPACITY_2] << 16;
	
	/* Supports DMA. */
	if (ata_info_supports_dma(dev->rawinfo))
		dev->flags |= ATADEV_DMA;
	
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
 * @brief Initializes the generic ATA driver.
 */
PUBLIC void ata_init(void)
{
	int i;     /* Loop index.    */
	char dvrl; /* Device letter. */
	
	/* Detect devices. */
	for (i = 0, dvrl = 'a'; i < 4; i++, dvrl++)
	{
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
					kprintf("hd%c: %d sectors.", dvrl, ata_devices[i].nsectors);
				}
				break;

			/* UNKNOWN. */
			case ATADEV_UNKNOWN:
				kprintf("hd?: unknown ATA device.");
				break;
		}
	}
}
