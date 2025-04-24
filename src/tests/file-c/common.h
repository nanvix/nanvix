/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _COMMON_H_
#define _COMMON_H_

// Tests system calls on directory entries.
extern void test_dirent(void);

// Tests wether we can open and close a file.
extern void test_open_close(void);

// Tests wether we can create and unlink a file.
extern void test_create_unlink(void);

// Tests whether we can get the current working directory.
extern void test_getcwd(void);

// Tests whether we can change the current working directory.
extern void test_fchdir(void);

// Tests whether we can change access permissions of a file.
extern void test_fchmod(void);

// Tests whether we can rename a file.
extern void test_renameat(void);

// Tests whether we can get file status information.
extern void test_stat(void);

// Tests whether we can remove a file.
extern void test_unlinkat(void);

// Tests whether we can change file access and modification times.
extern void test_utimensat(void);

#endif
