/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SEMAPHORE_H
#define _NANVIX_SEMAPHORE_H

/**
 * @file semaphore.h
 * @brief Semaphores.
 *
 * Declares the POSIX unnamed semaphore type and operations. The semaphore is
 * built on top of the kernel mutex and condition variable primitives; its layout
 * mirrors the Rust definition in the sysapi crate (sys_types.rs).
 */

#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief Unnamed semaphore. */
typedef struct {
    int __count;            /**< Current value of the semaphore.          */
    pthread_mutex_t __lock; /**< Mutex that guards the semaphore state.    */
    pthread_cond_t __cond;  /**< Condition variable used to block waiters. */
} sem_t;

/*==================================================================================================
 * Semaphores
 *==================================================================================================*/

extern int sem_init(sem_t *sem, int pshared, unsigned int value);
extern int sem_destroy(sem_t *sem);
extern int sem_post(sem_t *sem);
extern int sem_wait(sem_t *sem);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SEMAPHORE_H */
