/*
 * Copyright(C) 2011-2014 Pedro h. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <fcntl.h> - File control options.
 */

#ifndef FCNTL_H_
#define FCNTL_H_

	/* Access modes. */
	#define O_ACCMODE 00003 /* Mask.           */
	#define O_RDONLY     00 /* Read only.      */
	#define O_WRONLY     01 /* Write only.     */
	#define O_RDWR       02 /* Read and write. */

	/* File creation flags. */
	#define O_CREAT  00100 /* Create file if it does not exist.   */
	#define O_EXCL	 00200 /* Exclusive use flag.                 */
	#define O_NOCTTY 00400 /* Do not assign controlling terminal. */
	#define O_TRUNC	 01000 /* Truncate file.                      */
	
	/* File status flags. */
	#define O_APPEND 02000 /* Append mode. */
		
	/*
	 * Returns file's access mode.
	 */
	#define ACCMODE(m) (m & O_ACCMODE)
	
	/*
	 * Opens a file.
	 */
	extern int open(const char *path, int oflag, ...);
	
	/*
	 * Creates a file. 
	 */
	#define creat(path, mode) open(path, O_WRONLY | O_CREAT | O_TRUNC, mode)

#endif /* FCNTL_H_ */
