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
	#define O_APPEND   02000 /* Append mode.       */
	#define O_NONBLOCK 04000 /* Non-blocking mode. */
	
	/* File descriptor flags. */
	#define FD_CLOEXEC 01 /* Close on exec. */
		
	/* Commands for fcntl(). */
	#define F_DUPFD  0 /* Duplicate file descriptor.                   */
	#define F_GETFD  1 /* Get file descriptor flags.                   */
	#define F_SETFD  2 /* Set file descriptor flags.                   */
	#define F_GETFL  3 /* Get file status flags and file access modes. */
	#define F_SETFL  4 /* Set file status flags.                       */

	/*
	 * Returns file's access mode.
	 */
	#define ACCMODE(m) (m & O_ACCMODE)
	
	/*
	 * Opens a file.
	 */
	extern int open(const char *path, int oflag, ...);
	
	/*
	 * Manipulates file descriptor.
	 */
	extern int fcntl(int fd, int cmd, ...);
	
	/*
	 * Creates a file. 
	 */
	#define creat(path, mode) open(path, O_WRONLY | O_CREAT | O_TRUNC, mode)

#endif /* FCNTL_H_ */
