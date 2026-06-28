/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_STDATOMIC_H
#define _NANVIX_STDATOMIC_H

/**
 * @file stdatomic.h
 * @brief Atomic operations (C11 <stdatomic.h>).
 *
 * Freestanding header vendored in-tree so the guest C toolchain does not depend
 * on the compiler's builtin resource-directory headers. The atomic operations
 * are expressed with compiler atomic builtins: Clang's `__c11_atomic_*` (which
 * operate on `_Atomic`-qualified objects) and GCC's generic `__atomic_*`. Both
 * are provided for the active target, so the layouts and orderings always match
 * the selected ABI.
 */

#include <stddef.h>
#include <stdint.h>

/*==================================================================================================
 * Memory ordering
 *==================================================================================================*/

/** @brief Memory ordering constraints for atomic operations. */
typedef enum memory_order {
    memory_order_relaxed = __ATOMIC_RELAXED,
    memory_order_consume = __ATOMIC_CONSUME,
    memory_order_acquire = __ATOMIC_ACQUIRE,
    memory_order_release = __ATOMIC_RELEASE,
    memory_order_acq_rel = __ATOMIC_ACQ_REL,
    memory_order_seq_cst = __ATOMIC_SEQ_CST
} memory_order;

/*==================================================================================================
 * Lock-free property macros
 *==================================================================================================*/

#define ATOMIC_BOOL_LOCK_FREE __GCC_ATOMIC_BOOL_LOCK_FREE
#define ATOMIC_CHAR_LOCK_FREE __GCC_ATOMIC_CHAR_LOCK_FREE
#define ATOMIC_CHAR16_T_LOCK_FREE __GCC_ATOMIC_CHAR16_T_LOCK_FREE
#define ATOMIC_CHAR32_T_LOCK_FREE __GCC_ATOMIC_CHAR32_T_LOCK_FREE
#define ATOMIC_WCHAR_T_LOCK_FREE __GCC_ATOMIC_WCHAR_T_LOCK_FREE
#define ATOMIC_SHORT_LOCK_FREE __GCC_ATOMIC_SHORT_LOCK_FREE
#define ATOMIC_INT_LOCK_FREE __GCC_ATOMIC_INT_LOCK_FREE
#define ATOMIC_LONG_LOCK_FREE __GCC_ATOMIC_LONG_LOCK_FREE
#define ATOMIC_LLONG_LOCK_FREE __GCC_ATOMIC_LLONG_LOCK_FREE
#define ATOMIC_POINTER_LOCK_FREE __GCC_ATOMIC_POINTER_LOCK_FREE

/*==================================================================================================
 * Atomic types
 *==================================================================================================*/

typedef _Atomic(_Bool) atomic_bool;
typedef _Atomic(char) atomic_char;
typedef _Atomic(signed char) atomic_schar;
typedef _Atomic(unsigned char) atomic_uchar;
typedef _Atomic(short) atomic_short;
typedef _Atomic(unsigned short) atomic_ushort;
typedef _Atomic(int) atomic_int;
typedef _Atomic(unsigned int) atomic_uint;
typedef _Atomic(long) atomic_long;
typedef _Atomic(unsigned long) atomic_ulong;
typedef _Atomic(long long) atomic_llong;
typedef _Atomic(unsigned long long) atomic_ullong;
typedef _Atomic(__CHAR16_TYPE__) atomic_char16_t;
typedef _Atomic(__CHAR32_TYPE__) atomic_char32_t;
typedef _Atomic(__WCHAR_TYPE__) atomic_wchar_t;
typedef _Atomic(int_least8_t) atomic_int_least8_t;
typedef _Atomic(uint_least8_t) atomic_uint_least8_t;
typedef _Atomic(int_least16_t) atomic_int_least16_t;
typedef _Atomic(uint_least16_t) atomic_uint_least16_t;
typedef _Atomic(int_least32_t) atomic_int_least32_t;
typedef _Atomic(uint_least32_t) atomic_uint_least32_t;
typedef _Atomic(int_least64_t) atomic_int_least64_t;
typedef _Atomic(uint_least64_t) atomic_uint_least64_t;
typedef _Atomic(int_fast8_t) atomic_int_fast8_t;
typedef _Atomic(uint_fast8_t) atomic_uint_fast8_t;
typedef _Atomic(int_fast16_t) atomic_int_fast16_t;
typedef _Atomic(uint_fast16_t) atomic_uint_fast16_t;
typedef _Atomic(int_fast32_t) atomic_int_fast32_t;
typedef _Atomic(uint_fast32_t) atomic_uint_fast32_t;
typedef _Atomic(int_fast64_t) atomic_int_fast64_t;
typedef _Atomic(uint_fast64_t) atomic_uint_fast64_t;
typedef _Atomic(intptr_t) atomic_intptr_t;
typedef _Atomic(uintptr_t) atomic_uintptr_t;
typedef _Atomic(size_t) atomic_size_t;
typedef _Atomic(ptrdiff_t) atomic_ptrdiff_t;
typedef _Atomic(intmax_t) atomic_intmax_t;
typedef _Atomic(uintmax_t) atomic_uintmax_t;

/*==================================================================================================
 * Builtin selection (Clang `__c11_atomic_*` vs GCC `__atomic_*`)
 *==================================================================================================*/

#if defined(__clang__)
#define __nvx_atomic_init(obj, val) __c11_atomic_init((obj), (val))
#define __nvx_atomic_load(obj, ord) __c11_atomic_load((obj), (ord))
#define __nvx_atomic_store(obj, val, ord) __c11_atomic_store((obj), (val), (ord))
#define __nvx_atomic_exchange(obj, val, ord) __c11_atomic_exchange((obj), (val), (ord))
#define __nvx_atomic_cmpxchg_strong(obj, exp, des, s, f) \
    __c11_atomic_compare_exchange_strong((obj), (exp), (des), (s), (f))
#define __nvx_atomic_cmpxchg_weak(obj, exp, des, s, f) \
    __c11_atomic_compare_exchange_weak((obj), (exp), (des), (s), (f))
#define __nvx_atomic_fetch_add(obj, op, ord) __c11_atomic_fetch_add((obj), (op), (ord))
#define __nvx_atomic_fetch_sub(obj, op, ord) __c11_atomic_fetch_sub((obj), (op), (ord))
#define __nvx_atomic_fetch_or(obj, op, ord) __c11_atomic_fetch_or((obj), (op), (ord))
#define __nvx_atomic_fetch_xor(obj, op, ord) __c11_atomic_fetch_xor((obj), (op), (ord))
#define __nvx_atomic_fetch_and(obj, op, ord) __c11_atomic_fetch_and((obj), (op), (ord))
#define __nvx_atomic_thread_fence(ord) __c11_atomic_thread_fence((ord))
#define __nvx_atomic_signal_fence(ord) __c11_atomic_signal_fence((ord))
#define __nvx_atomic_is_lock_free(obj) __c11_atomic_is_lock_free(sizeof(*(obj)))
#else
#define __nvx_atomic_init(obj, val) __atomic_store_n((obj), (val), __ATOMIC_RELAXED)
#define __nvx_atomic_load(obj, ord) __atomic_load_n((obj), (ord))
#define __nvx_atomic_store(obj, val, ord) __atomic_store_n((obj), (val), (ord))
#define __nvx_atomic_exchange(obj, val, ord) __atomic_exchange_n((obj), (val), (ord))
#define __nvx_atomic_cmpxchg_strong(obj, exp, des, s, f) \
    __atomic_compare_exchange_n((obj), (exp), (des), 0, (s), (f))
#define __nvx_atomic_cmpxchg_weak(obj, exp, des, s, f) \
    __atomic_compare_exchange_n((obj), (exp), (des), 1, (s), (f))
#define __nvx_atomic_fetch_add(obj, op, ord) __atomic_fetch_add((obj), (op), (ord))
#define __nvx_atomic_fetch_sub(obj, op, ord) __atomic_fetch_sub((obj), (op), (ord))
#define __nvx_atomic_fetch_or(obj, op, ord) __atomic_fetch_or((obj), (op), (ord))
#define __nvx_atomic_fetch_xor(obj, op, ord) __atomic_fetch_xor((obj), (op), (ord))
#define __nvx_atomic_fetch_and(obj, op, ord) __atomic_fetch_and((obj), (op), (ord))
#define __nvx_atomic_thread_fence(ord) __atomic_thread_fence((ord))
#define __nvx_atomic_signal_fence(ord) __atomic_signal_fence((ord))
#define __nvx_atomic_is_lock_free(obj) __atomic_is_lock_free(sizeof(*(obj)), (obj))
#endif

/*==================================================================================================
 * Initialization
 *==================================================================================================*/

/** @brief Initializes a static atomic object (C17 deprecated; kept for compatibility). */
#define ATOMIC_VAR_INIT(value) (value)

/** @brief Initializes an existing atomic object. */
#define atomic_init(obj, value) __nvx_atomic_init((obj), (value))

/** @brief Breaks a dependency chain for memory_order_consume. */
#define kill_dependency(y) (y)

/*==================================================================================================
 * Fences
 *==================================================================================================*/

#define atomic_thread_fence(order) __nvx_atomic_thread_fence(order)
#define atomic_signal_fence(order) __nvx_atomic_signal_fence(order)

/*==================================================================================================
 * Lock-free queries
 *==================================================================================================*/

#define atomic_is_lock_free(obj) __nvx_atomic_is_lock_free(obj)

/*==================================================================================================
 * Generic atomic operations
 *==================================================================================================*/

#define atomic_store_explicit(object, desired, order) \
    __nvx_atomic_store((object), (desired), (order))
#define atomic_store(object, desired) \
    atomic_store_explicit((object), (desired), memory_order_seq_cst)

#define atomic_load_explicit(object, order) __nvx_atomic_load((object), (order))
#define atomic_load(object) atomic_load_explicit((object), memory_order_seq_cst)

#define atomic_exchange_explicit(object, desired, order) \
    __nvx_atomic_exchange((object), (desired), (order))
#define atomic_exchange(object, desired) \
    atomic_exchange_explicit((object), (desired), memory_order_seq_cst)

#define atomic_compare_exchange_strong_explicit(object, expected, desired, success, failure) \
    __nvx_atomic_cmpxchg_strong((object), (expected), (desired), (success), (failure))
#define atomic_compare_exchange_strong(object, expected, desired) \
    atomic_compare_exchange_strong_explicit((object), (expected), (desired), memory_order_seq_cst, \
                                            memory_order_seq_cst)

#define atomic_compare_exchange_weak_explicit(object, expected, desired, success, failure) \
    __nvx_atomic_cmpxchg_weak((object), (expected), (desired), (success), (failure))
#define atomic_compare_exchange_weak(object, expected, desired) \
    atomic_compare_exchange_weak_explicit((object), (expected), (desired), memory_order_seq_cst, \
                                          memory_order_seq_cst)

#define atomic_fetch_add_explicit(object, operand, order) \
    __nvx_atomic_fetch_add((object), (operand), (order))
#define atomic_fetch_add(object, operand) \
    atomic_fetch_add_explicit((object), (operand), memory_order_seq_cst)

#define atomic_fetch_sub_explicit(object, operand, order) \
    __nvx_atomic_fetch_sub((object), (operand), (order))
#define atomic_fetch_sub(object, operand) \
    atomic_fetch_sub_explicit((object), (operand), memory_order_seq_cst)

#define atomic_fetch_or_explicit(object, operand, order) \
    __nvx_atomic_fetch_or((object), (operand), (order))
#define atomic_fetch_or(object, operand) \
    atomic_fetch_or_explicit((object), (operand), memory_order_seq_cst)

#define atomic_fetch_xor_explicit(object, operand, order) \
    __nvx_atomic_fetch_xor((object), (operand), (order))
#define atomic_fetch_xor(object, operand) \
    atomic_fetch_xor_explicit((object), (operand), memory_order_seq_cst)

#define atomic_fetch_and_explicit(object, operand, order) \
    __nvx_atomic_fetch_and((object), (operand), (order))
#define atomic_fetch_and(object, operand) \
    atomic_fetch_and_explicit((object), (operand), memory_order_seq_cst)

/*==================================================================================================
 * Atomic flag
 *==================================================================================================*/

/** @brief Lock-free atomic boolean flag. */
typedef struct atomic_flag {
    _Atomic(_Bool) __value;
} atomic_flag;

/** @brief Initializer for an atomic_flag in the clear state. */
#define ATOMIC_FLAG_INIT \
    {                  \
        0              \
    }

#define atomic_flag_test_and_set_explicit(object, order) \
    __nvx_atomic_exchange(&(object)->__value, 1, (order))
#define atomic_flag_test_and_set(object) \
    atomic_flag_test_and_set_explicit((object), memory_order_seq_cst)

#define atomic_flag_clear_explicit(object, order) __nvx_atomic_store(&(object)->__value, 0, (order))
#define atomic_flag_clear(object) atomic_flag_clear_explicit((object), memory_order_seq_cst)

#endif /* _NANVIX_STDATOMIC_H */
