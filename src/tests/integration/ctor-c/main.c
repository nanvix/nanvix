/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * Validates the `.init_array` constructor framing walked by
 * `__nanvix_libc_start_main` (doc/toolchain-migration.md §4.5, decision 1).
 *
 * Nanvix ships no GCC `crtbegin`/`crtend`; instead the libc-side start-of-day
 * driver walks `.preinit_array` and `.init_array` (whose bounds come from the
 * guest `user.ld`) before calling `main`. This suite asserts that:
 *
 *   1. a default-priority global constructor runs before `main`, and
 *   2. prioritized constructors run in ascending-priority order
 *      (exercising the `SORT_BY_INIT_PRIORITY` collection in `user.ld`).
 *
 * The suite passes (returns 0) only when both hold.
 */

static int ctor_default_ran = 0;
static int ctor_prio_seq[2] = {0, 0};
static int ctor_prio_pos = 0;

__attribute__((constructor)) static void ctor_default(void)
{
    ctor_default_ran = 1;
}

__attribute__((constructor(101))) static void ctor_first(void)
{
    if (ctor_prio_pos < 2)
    {
        ctor_prio_seq[ctor_prio_pos++] = 101;
    }
}

__attribute__((constructor(102))) static void ctor_second(void)
{
    if (ctor_prio_pos < 2)
    {
        ctor_prio_seq[ctor_prio_pos++] = 102;
    }
}

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    /* The default-priority constructor must have run before main. */
    if (!ctor_default_ran)
    {
        return (1);
    }

    /* Prioritized constructors must have run, lower priority first. */
    if (ctor_prio_seq[0] != 101 || ctor_prio_seq[1] != 102)
    {
        return (2);
    }

    return (0);
}
