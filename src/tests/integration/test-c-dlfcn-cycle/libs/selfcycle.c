/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * libselfcycle.so - single-node DT_NEEDED self-loop (libselfcycle.so
 * DT_NEEDED libselfcycle.so). Exercises the loader's direct self-cycle
 * branch, where a library's own dependency resolves back to itself.
 *
 * The self-edge is forced at link time by listing this library's own
 * stage-1 image on its final link line, not by any symbol reference, so a
 * single dependency-free leaf function is all this source needs to provide.
 */

int selfcycle_value(void)
{
    return 0x5E1F;
}
