/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_FCNTL_H
#define _NANVIX_FCNTL_H

/**
 * @file fcntl.h
 * @brief File control options.
 *
 * Declares the file-control flags and the open()/fcntl()-family interfaces. The
 * constants mirror the Rust definitions in the sysapi crate (fcntl.rs).
 */

#include <sys/types.h>
#include <sys/stat.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#define O_ACCMODE 0x3 /**< Mask for file access mode. */
#define O_RDONLY 0 /**< Open for reading only. */
#define O_WRONLY 1 /**< Open for writing only. */
#define O_RDWR 2 /**< Open for reading and writing. */
#define O_APPEND 0x0008 /**< Set append mode. */
#define O_CREAT 0x0200 /**< Create file if it does not exist. */
#define O_TRUNC 0x0400 /**< Truncate file to size zero. */
#define O_EXCL 0x0800 /**< Fail if file already exists. */
#define O_SYNC 0x2000 /**< Synchronized I/O data integrity. */
#define O_NONBLOCK 0x4000 /**< Non-blocking mode. */
#define O_NOCTTY 0x8000 /**< Do not assign controlling terminal. */
#define O_CLOEXEC 0x40000 /**< Close-on-exec. */
#define O_CLOFORK 0x80000 /**< Close-on-fork. */
#define O_NOFOLLOW 0x100000 /**< Do not follow symbolic links. */
#define O_DIRECTORY 0x200000 /**< Fail if not a directory. */
#define AT_FDCWD -100 /**< Use the current working directory. */
#define AT_EACCESS 1 /**< Check access using effective IDs. */
#define AT_SYMLINK_NOFOLLOW 2 /**< Do not follow symbolic links. */
#define AT_REMOVEDIR 8 /**< Remove a directory instead of a file. */
#define POSIX_FADV_NORMAL 0 /**< No advice. */
#define POSIX_FADV_SEQUENTIAL 1 /**< Sequential access. */
#define POSIX_FADV_RANDOM 2 /**< Random access. */
#define POSIX_FADV_WILLNEED 3 /**< Will be accessed soon. */
#define POSIX_FADV_DONTNEED 4 /**< Will not be accessed soon. */
#define POSIX_FADV_NOREUSE 5 /**< Accessed once. */

/*==================================================================================================
 * File Control
 *==================================================================================================*/

/* fcntl() commands. */
#define F_DUPFD 0
#define F_GETFD 1
#define F_SETFD 2
#define F_GETFL 3
#define F_SETFL 4
#define F_GETLK 5
#define F_SETLK 6
#define F_SETLKW 7

/* File-descriptor flags (F_GETFD/F_SETFD). */
#define FD_CLOEXEC 1
#define FD_CLOFORK 2

/* Lock types (struct flock l_type). */
#define F_RDLCK 0
#define F_WRLCK 1
#define F_UNLCK 2

/** @brief Advisory record lock. */
struct flock {
    short l_type;   /**< Lock type: F_RDLCK, F_WRLCK, F_UNLCK. */
    short l_whence; /**< How to interpret l_start.            */
    off_t l_start;  /**< Starting offset for the lock.        */
    off_t l_len;    /**< Number of bytes to lock.             */
    pid_t l_pid;    /**< PID of the process holding the lock. */
};

extern int open(const char *path, int flags, ...);
extern int openat(int dirfd, const char *path, int flags, ...);
extern int fcntl(int fd, int cmd, ...);
extern int posix_fadvise(int fd, off_t offset, off_t len, int advice);
extern int posix_fallocate(int fd, off_t offset, off_t len);
extern int renameat(int olddirfd, const char *oldpath, int newdirfd, const char *newpath);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_FCNTL_H */
