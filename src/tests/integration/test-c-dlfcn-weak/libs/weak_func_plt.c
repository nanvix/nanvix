/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * Companion to libs/weak_func.c that exercises the *PLT* path for weak
 * undefined function references.
 *
 * Calling the weak symbol without a preceding `&fn != NULL` check lets the
 * compiler route the call through the PLT, producing a `R_386_JUMP_SLOT`
 * relocation rather than the GOT-indirect `R_386_GLOB_DAT` emitted by
 * libs/weak_func.c. Together the two helpers cover both relocation classes
 * the dynamic loader must handle for weak undefined symbols.
 *
 * Safety note: the `missing` variant of this helper produces a `.so` whose
 * weak symbol resolves to address zero at runtime. The main test driver
 * MUST NOT invoke `call_weak_unchecked` against that variant -- doing so
 * would jump to NULL. Resolving `dlsym(handle, "call_weak_unchecked")` is
 * safe; the goal of the missing variant is only to prove that the loader
 * accepts the `.so` under RTLD_NOW despite the weak undefined PLT entry.
 */

#ifndef CALLBACK_NAME
#define CALLBACK_NAME main_callback
#endif

extern int CALLBACK_NAME(int) __attribute__((weak));

int call_weak_unchecked(int x)
{
    return (CALLBACK_NAME(x));
}
