/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <ustat.h> - File system statistics structure.
 */

#ifndef USTAT_H_
#define USTAT_H_
#ifndef _ASM_FILE_

	#include <sys/types.h>

	/*
	 * File system statistics.
	 */
	struct ustat
	{
		daddr_t f_tfree; /* Total free blocks.     */
		ino_t f_tinode;  /* Number of free inodes. */
		char f_fname[6]; /* File system name.      */
		char f_fpack[6]; /* File system pack name. */
	};
	
	/*
	 * Gets file system statistics.
	 */
	extern int ustat(dev_t dev, struct ustat *ubuf);

#endif /* _ASM_FILE_ */
#endif /* USTAT_H_ */
