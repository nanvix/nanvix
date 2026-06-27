/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _COMMON_H_
#define _COMMON_H_

// Tests whether we can check access permissions of a file using access().
extern void test_access(void);

// Tests whether we can change the current working directory.
extern void test_chdir(void);

// Tests system calls on directory entries.
extern void test_dirent(void);

// Tests whether we can open and close a file.
extern void test_open_close(void);

// Tests whether we can create and unlink a file.
extern void test_create_unlink(void);

// Tests whether we can get the current working directory.
extern void test_getcwd(void);

// Tests whether we can check access permissions of a file.
extern void test_faccessat(void);

// Tests whether we can change the current working directory.
extern void test_fchdir(void);

// Tests whether we can change access permissions of a file.
extern void test_chmod(void);

// Tests whether we can change the ownership of a file.
extern void test_chown(void);

// Tests whether we can change access permissions of a file.
extern void test_fchmod(void);

// Tests whether we can change access permissions of a file.
extern void test_fchmodat(void);

// Tests whether we can change the ownership of a file descriptor.
extern void test_fchown(void);

// Tests whether we can change the ownership of a file.
extern void test_fchownat(void);

// Tests whether we can synchronize file data to disk.
extern void test_fdatasync(void);

// Tests whether we can truncate a file descriptor.
extern void test_ftruncate(void);

// Tests whether we can change access permissions of a file.
extern void test_lchmod(void);

// Tests whether we can change the ownership of a file.
extern void test_lchown(void);

// Tests whether we can create a hard link to a file.
extern void test_link(void);

// Tests whether we can create a hard link to a file.
extern void test_linkat(void);

// Tests whether we can manipulate file offsets using lseek.
extern void test_lseek(void);

// Tests whether we can update file timestamps with `futimens()`.
extern void test_futimens(void);

// Tests whether we can create a directory.
extern void test_mkdirat(void);

// Tests whether we can create a directory.
extern void test_mkdir(void);

// Tests whether we can use posix_fadvise on a file.
extern void test_posix_fadvise(void);

// Tests whether we can use posix_fallocate on a file.
extern void test_posix_fallocate(void);

// Tests whether we can read from a file using pread.
extern void test_pread(void);

// Tests whether we can read from a file using preadv.
extern void test_preadv(void);

// Tests whether we can write to a file using pwrite.
extern void test_pwrite(void);

// Tests whether we can write to a file using pwritev.
extern void test_pwritev(void);

// Tests whether we can read a symbolic link.
extern void test_readlink(void);

// Tests whether we can read a symbolic link.
extern void test_readlinkat(void);

// Tests whether we can read from a file using vectorized I/O.
extern void test_readv(void);

// Tests whether we can rename a file.
extern void test_renameat(void);

// Tests whether we can get file status information.
extern void test_stat(void);

// Tests whether we can create a symbolic link to a file.
extern void test_symlinkat(void);

// Tests whether we can remove a file.
extern void test_unlinkat(void);

// Tests whether we can change file access and modification times.
extern void test_utimensat(void);

// Tests whether we can update file timestamps with `utimes()`.
extern void test_utimes(void);

// Tests whether we can update file timestamps with `utime()`.
extern void test_utime(void);

// Tests whether we can write and read to/from a file.
extern void test_write_read(void);

// Tests whether we can write to a file using vectorized I/O.
extern void test_writev(void);

// Tests whether we can poll a file descriptor for read/write readiness.
extern void test_poll(void);

// Tests whether we can use select() to check read/write readiness on a file descriptor.
extern void test_select(void);

#endif
