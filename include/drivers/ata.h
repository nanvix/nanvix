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

#ifndef ATA_H_
#define ATA_H_

	#include <stdint.h>
	
	/* ATA drives. */
	#define ATA_PRI_MASTER 0 /* Primary master.   */
	#define ATA_PRI_SLAVE  1 /* Primary slave.    */
	#define ATA_SEC_MASTER 2 /* Secondary master. */
	#define ATA_SEC_SLAVE  3 /* Secondary slave.  */
	
	/* ATA Buses. */
	#define ATA_BUS_PRIMARY   0 /* Primary bus.   */
	#define ATA_BUS_SECONDARY 1 /* Secondary bus. */
	
	/* ATA controller registers. */
	#define ATA_REG_DATA    0 /* Data register.             */
	#define ATA_REG_ERR     1 /* Error register.            */
	#define ATA_REG_FEATURE 1 /* Features register.         */
	#define ATA_REG_NSECT   2 /* Sector count register.     */
	#define ATA_REG_LBAL    3 /* LBA low register.          */
	#define ATA_REG_LBAM    4 /* LBA middle register.       */
	#define ATA_REG_LBAH    5 /* LBA high register.         */
	#define ATA_REG_DEVICE  6 /* Device register.           */
	#define ATA_REG_DEVCTL  6 /* Device control register.   */
	#define ATA_REG_CMD     7 /* Command register.          */
	#define ATA_REG_STATUS  7 /* Status register.           */
	#define ATA_REG_ASTATUS 8 /* Alternate status register. */
	
	/* ATA status register. */
	#define ATA_ERR   (1 << 0) /* Device error. */
	#define ATA_DRQ   (1 << 3) /* Data request. */
	#define ATA_DF    (1 << 5) /* Device fault. */
	#define ATA_READY (1 << 6) /* Device ready. */
	#define ATA_BUSY  (1 << 7) /* Device busy.  */

	/* ATA commands. */
	#define ATA_CMD_RESET				0x08
	#define ATA_CMD_IDENTIFY			0xEC /* Identify device. */
	#define ATA_CMD_READ_SECTORS		0x20
	#define ATA_CMD_READ_SECTORS_EXT	0x24
	#define ATA_CMD_WRITE_SECTORS		0x30
	#define ATA_CMD_WRITE_SECTORS_EXT	0x34
	#define ATA_CMD_FLUSH_CACHE			0xE7
	#define ATA_CMD_FLUSH_CACHE_EXT		0xEA
	
	/* ATA device information. */
	#define ATA_INFO_WORDS                 256
	#define ATA_INFO_CONFIG                  0 /* Configuration. */
	#define ATA_INFO_CYLS                    1
	#define ATA_INFO_HEADS                   3
	#define ATA_INFO_SECTORS                 6
	#define ATA_INFO_SERNO                  10
	#define ATA_INFO_BUF_SIZE               21
	#define ATA_INFO_FW_REV                 23
	#define ATA_INFO_PROD                   27
	#define ATA_INFO_MAX_MULTSECT           47
	#define ATA_INFO_DWORD_IO               48
	#define ATA_INFO_CAPABILITY_1           49 /* Capabilities 0. */
	#define ATA_INFO_CAPABILITY_2           50 /* Capabilities 1. */
	#define ATA_INFO_OLD_PIO_MODES          51
	#define ATA_INFO_OLD_DMA_MODES          52
	#define ATA_INFO_FIELD_VALINFO          53
	#define ATA_INFO_CUR_CYLS               54
	#define ATA_INFO_CUR_HEADS              55
	#define ATA_INFO_CUR_SECTORS            56
	#define ATA_INFO_MULTSECT               59
	#define ATA_INFO_LBA_CAPACITY_1         60 /* LBA addressable sectors 0. */
	#define ATA_INFO_LBA_CAPACITY_2         61 /* LBA addressable sectors 1. */
	#define ATA_INFO_SWDMA_MODES            62
	#define ATA_INFO_MWDMA_MODES            63
	#define ATA_INFO_PIO_MODES              64
	#define ATA_INFO_EINFOE_DMA_MIN         65
	#define ATA_INFO_EINFOE_DMA_TIME        66
	#define ATA_INFO_EINFOE_PIO             67
	#define ATA_INFO_EINFOE_PIO_IORDY       68
	#define ATA_INFO_ADDITIONAL_SUPP        69
	#define ATA_INFO_QUEUE_DEPTH            75
	#define ATA_INFO_SATA_CAPABILITY_1      76
	#define ATA_INFO_SATA_CAPABILITY_2      77
	#define ATA_INFO_FEATURE_SUPP           78
	#define ATA_INFO_MAJOR_VER              80
	#define ATA_INFO_COMMAND_SET_1          82
	#define ATA_INFO_COMMAND_SET_2          83
	#define ATA_INFO_CFSSE                  84
	#define ATA_INFO_CFS_ENABLE_1           85
	#define ATA_INFO_CFS_ENABLE_2           86
	#define ATA_INFO_CSF_DEFAULT            87
	#define ATA_INFO_UDMA_MODES             88
	#define ATA_INFO_HW_CONFIG              93
	#define ATA_INFO_SPG                    98
	#define ATA_INFO_LBA48_CAPACITY        100
	#define ATA_INFO_SECTOR_SIZE           106
	#define ATA_INFO_WWN                   108
	#define ATA_INFO_LOGICAL_SECTOR_SIZE_1 117	
	#define ATA_INFO_LOGICAL_SECTOR_SIZE_2 117
	#define ATA_INFO_LAST_LUN              126
	#define ATA_INFO_DLF                   128
	#define ATA_INFO_CSFO                  129
	#define ATA_INFO_CFA_POWER             160
	#define ATA_INFO_CFA_KEY_MGMT          162
	#define ATA_INFO_CFA_MODES             163
	#define ATA_INFO_DATA_SET_MGMT         169
	#define ATA_INFO_ROT_SPEED             217
	
	/**
	 * @brief Asserts if ATA device is a ATA device.
	 */
	#define ata_info_is_ata(info) \
		(((info)[ATA_INFO_CONFIG] & (1 << 15)) == 0)
	
	/**
	 * @brief Asserts if ATA device supports LBA.
	 */
	#define ata_info_supports_lba(info) \
		((info)[ATA_INFO_CAPABILITY_1] & (1 << 9))
		
	/**
	 * @briefs Asserts if ATA device supports DMA.
	 */
	#define ata_info_supports_dma(info) \
		((info)[ATA_INFO_CAPABILITY_1] & (1 << 8))
	
	/**
	 * @brief Returns the bus number of a ATA drive.
	 */
	#define ATA_BUS(x) \
		(((x) < 2) ? ATA_BUS_PRIMARY : ATA_BUS_SECONDARY)
	
	/* ATA device types. */
	#define ATADEV_NULL    0 /* Null.                          */
	#define ATADEV_UNKNOWN 1 /* Unknown.                       */
	#define ATADEV_PATAPI  2 /* Parallel ATA Packet Interface. */
	#define ATADEV_SATAPI  3 /* Serial ATA Packet Interface.   */
	#define ATADEV_PATA    4 /* Parallel ATA.                  */
	#define ATADEV_SATA    5 /* Serial ATA.                    */
	
	
	/* ATA device flags. */
	#define ATADEV_PRESENT (1 << 0) /* Present.     */
	#define ATADEV_LBA     (1 << 1) /* LBA support. */
	#define ATADEV_DMA     (1 << 2) /* DMA support. */

	/**
	 * @brief ATA device information.
	 */
	struct atadev
	{
		int flags;                        /* Flags (see above).       */
		int nsectors;                     /* Number of sectors.       */
		int type;                         /* Device type (see above). */
		uint16_t rawinfo[ATA_INFO_WORDS]; /* Raw information.         */
	};

#endif /* ATA_H_ */
