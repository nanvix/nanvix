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
#include <nanvix/const.h>
#include <nanvix/klib.h>

/* Buses. */
#define PRI_BUS   0 /* Primary.   */
#define SEC_BUS 1 /* Secondary. */

/* Devices. */
#define MASTER_DEV 0 /* Master. */
#define SLAVE_DEV  1 /* Slave.  */

/* ATA controller registers. */
#define REG_DATA	0 /* Used to read/write PIO data.                      */
#define REG_FERR	1 /* Features and error information.                   */
#define REG_SC		2 /* Sector count, number of sectors to read/write.    */
#define REG_SADDR1	3 /* Sector number.                                    */
#define REG_SADDR2	4 /* Partial disk sector address.                      */
#define REG_SADDR3	5 /* Partial disk sector address.                      */
#define REG_DC		6 /* Used to select a drive and/or head.               */
#define REG_CMD		7 /* Used to send commands or read the current status. */
#define REG_ASTATUS	8 /* Status.                                           */

/* ATA commands. */
#define CMD_RESET				0x08
#define CMD_IDENTIFY			0xEC /* Identify device. */
#define CMD_READ_SECTORS		0x20
#define CMD_READ_SECTORS_EXT	0x24
#define CMD_WRITE_SECTORS		0x30
#define CMD_WRITE_SECTORS_EXT	0x34
#define CMD_FLUSH_CACHE			0xE7
#define CMD_FLUSH_CACHE_EXT		0xEA

/* Device status byte. */
#define ERR_BIT 0 /* An error occurred.                 */
#define DRQ_BIT 3 /* Ready to accept/transfer PIO data. */
#define SRV_BIT 4 /* Overlapped Mode Service Request.   */
#define DF_BIT  5 /* Fault error.                       */
#define RDY_BIT 6 /* Ready.                             */
#define BSY_BIT 7 /* Busy.                              */

/**
 * @brief Default I/O ports for ATA controller.
 */
PRIVATE uint16_t pio_ports[2][9] = {
		/* Primary Bus */
		{0x1F0, 0x1F1, 0x1F2, 0x1F3, 0x1F4, 0x1F5, 0x1F6, 0x1F7, 0x3F6},
		/* Secondary Bus */
		{0x170, 0x171, 0x172, 0x173, 0x174, 0x175, 0x176, 0x177, 0x376}
};

/**
 * @brief      Forces an I/O delay.
 * @param time Desired delay in nanoseconds.
 */
PRIVATE void delay(int time)
{
	int i;
	
	time /= 100;
	
	for (i = 0; i < time; i++)
		iowait();
}

/**
 * @brief        Select active device on the ATA bus
 * @param bus    Bus to be used (primary or secondary).
 * @param device Device to be selected (master or slave).
 */
PRIVATE void set_device(uint8_t bus, uint8_t device)
{
	/* Primary bus. */
	if (bus == PRI_BUS)
	{
		/* Master device. */
		if (device == MASTER_DEV)
			outputb(pio_ports[PRI_BUS][REG_DC], 0xa0);
		
		/* Slave device. */
		else if (device == SLAVE_DEV)
			outputb(pio_ports[PRI_BUS][REG_DC], 0xb0);
	}
	
	/* Secondary bus. */
	else if (bus == SEC_BUS)
	{
		/* Master device. */
		if (device == MASTER_DEV)
			outputb(pio_ports[SEC_BUS][REG_DC], 0xa0);
		
		/* Slave device. */
		else if (device == SLAVE_DEV)
			outputb(pio_ports[SEC_BUS][REG_DC], 0xb0);
	}
	
	delay(500);
}

/**
 * @brief Initializes the generic driver for ATA controllers.
 */
PUBLIC void ata_init(void)
{
	int i;          /* Loop index.    */
	char dvrl;      /* Driver letter. */
	int status;     /* Status byte.   */
	uint8_t bus;    /* Bus.           */
	uint8_t device; /* Device.        */
	uint8_t saddr2, saddr3;
	
	/* Detect devices. */
	bus = PRI_BUS;
	for (i = 0, dvrl = 'a'; i < 4; i++, dvrl++)
	{
		/* Switch to secondary bus. */
		if (i >= 2)
			bus = SEC_BUS;
		
		/* Select device. */
		device = ((i == 0) || (i == 2)) ? MASTER_DEV : SLAVE_DEV;
		set_device(bus, device);
		
		outputb(pio_ports[bus][REG_SC], 0);
		outputb(pio_ports[bus][REG_SADDR1], 0);
		outputb(pio_ports[bus][REG_SADDR2], 0);
		outputb(pio_ports[bus][REG_SADDR3], 0);
		
		outputb(pio_ports[bus][REG_CMD], CMD_IDENTIFY);
		
		/* No device attached. */
		if (inputb(pio_ports[bus][REG_ASTATUS]) == 0)
			continue;
	
		while (inputb(pio_ports[bus][REG_ASTATUS] & (1 << BSY_BIT)))
			/* noop*/ ;
		
		/* Check device type */
		saddr2 = inputb(pio_ports[bus][REG_SADDR2]);
		saddr3 = inputb(pio_ports[bus][REG_SADDR3]);
			
		/* ATAPI device.  */
		if ((saddr2 == 0x14) && (saddr3 == 0xeb))
			kprintf("hd%c: CD/DVD-ROM detected.", dvrl);
			
		/* SATAPI device. */
		else if ((saddr2 == 0x69) && (saddr3 == 0x96))
			kprintf("hd%c: CD/DVD-ROM detected.", dvrl);
			
		/* SATA device. */
		else if ((saddr2 == 0x3c) && (saddr3 == 0xc3))
			kprintf("hd%c: SATA HDD detected.", dvrl);

		/* Unkown device. */
		else if ((saddr2 != 0) && (saddr3 != 0))
			kprintf("hd?: %x %x.", saddr2, saddr3);

		/* ATA device. */
		else
		{
			kprintf("hd%c: ATA HDD detected.", dvrl);
			
			/* Probe device. */
			while (1)
			{
				status = inputb(pio_ports[bus][REG_ASTATUS]);
				
				/* Device is online. */
				if (status & (1 << DRQ_BIT))
				{
					kprintf("hd%c: online.", dvrl);
					break;
				}
					
				/* Error when probing. */
				if (status & (1 << ERR_BIT))
				{
					kprintf("hd%c: error.", dvrl);
					break;
				}
			}
		}
			
	}
}
