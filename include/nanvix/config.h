/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * config.h - Kernel configuration
 */

#ifndef CONFIG_H_
#define CONFIG_H_
	
	/* Machine configuration. */
	#define CPU              i386 /* Cpu model.   */
	#define MEMORY_SIZE 0x1000000 /* Memory size. */
	
	/* Kernel configuration. */
	#define PROC_MAX          64 /* Maximum number of process. */
	#define PROC_SIZE_MAX      8 /* Maximum process size.      */
	#define RAMDISK_SIZE 0x80000 /* RAM disks size.            */
	
#endif /* CONFIG_H_ */
