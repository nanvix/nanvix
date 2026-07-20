/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libother.so - self-contained library dlopen()'d with RTLD_GLOBAL.
 *
 * Exports scope_other_value(). Loading it with RTLD_GLOBAL publishes this
 * symbol into the loader's global scope, where RTLD_DEFAULT can see it. It has
 * NO dependency relationship with libfoo.so, so it must remain invisible to
 * dlsym(libfoo_handle, "scope_other_value"). The return value (333) is asserted
 * by the suite's main.c (OTHER_VALUE).
 */

int scope_other_value(void)
{
    return 333;
}
