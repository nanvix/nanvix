/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SYS_MMAN_H
#define _NANVIX_SYS_MMAN_H

/**
 * @file sys/mman.h
 * @brief Memory management declarations.
 *
 * Declares the memory-mapping protection and flag constants and the mmap/munmap
 * interfaces. Constants mirror the Rust definitions in the sysapi crate
 * (sys_mman.rs).
 */

#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#define PROT_NONE 0x0 /**< Page cannot be accessed. */
#define PROT_READ 0x1 /**< Page can be read. */
#define PROT_WRITE 0x2 /**< Page can be written. */
#define PROT_EXEC 0x4 /**< Page can be executed. */
#define MAP_SHARED 0x01 /**< Share changes. */
#define MAP_PRIVATE 0x02 /**< Changes are private. */
#define MAP_FIXED 0x10 /**< Interpret the address exactly. */
#define MAP_ANONYMOUS 0x20 /**< Anonymous mapping (not backed by a file). */

/** @brief Returned by mmap() on failure. */
#define MAP_FAILED ((void *)-1)

/*==================================================================================================
 * Memory Mapping
 *==================================================================================================*/

extern void *mmap(void *addr, size_t length, int prot, int flags, int fd, off_t offset);
extern int munmap(void *addr, size_t length);

/*==================================================================================================
 * Memory Protection
 *==================================================================================================*/

extern int mprotect(void *addr, size_t length, int prot);

/*==================================================================================================
 * Memory Locking
 *==================================================================================================*/

extern int mlock(const void *addr, size_t len);
extern int munlock(const void *addr, size_t len);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SYS_MMAN_H */
