/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_DLFCN_H
#define _NANVIX_DLFCN_H

/**
 * @file dlfcn.h
 * @brief Dynamic linking.
 *
 * Declares the dynamic-linking-loader constants, the symbol-information structure,
 * and the dlopen()-family interfaces. The constants and layout mirror the Rust
 * definitions in the posix and syscall crates (dlfcn).
 */

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#define RTLD_LAZY 0x1 /**< Relocations are performed at an implementation-defined time. */
#define RTLD_NOW 0x2 /**< Relocations are performed when the object is loaded. */
#define RTLD_GLOBAL 0x4 /**< Symbols are available for relocation processing of other objects. */
#define RTLD_LOCAL 0x0 /**< Symbols are not made available to other objects. */
#define RTLD_DEFAULT ((void *)0) /**< Pseudo-handle: search the global (default) symbol scope. */

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief Symbol information returned by dladdr(). */
typedef struct {
    const char *dli_fname; /**< Pathname of the mapped object.       */
    void *dli_fbase;       /**< Base address of the mapped object.   */
    const char *dli_sname; /**< Name of the nearest symbol.          */
    void *dli_saddr;       /**< Exact address of the symbol.         */
} Dl_info_t;

/*==================================================================================================
 * Dynamic Linking
 *==================================================================================================*/

extern void *dlopen(const char *filename, int mode);
extern void *dlsym(void *handle, const char *symbol);
extern int32_t dlclose(void *handle);
extern char *dlerror(void);
extern int32_t dladdr(const void *addr, Dl_info_t *dlip);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_DLFCN_H */
