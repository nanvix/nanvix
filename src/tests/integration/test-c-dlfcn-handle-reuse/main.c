/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * dlfcn-handle-reuse-c: Validate that a dlopen() handle is a stable, opaque
 * identifier that never aliases a different library, even after the underlying
 * file descriptor is recycled.
 *
 * Before the fix, a library's handle WAS its underlying file descriptor number.
 * File descriptor numbers are recycled by the kernel after close(), so a handle
 * to a closed library could silently alias a completely different library
 * loaded afterwards:
 *
 *   h1 = dlopen("libalpha.so");   // handle == fd (e.g. 5)
 *   dlclose(h1);                  // fd 5 freed
 *   h2 = dlopen("libbeta.so");    // fd 5 reused -> handle == 5 == h1
 *   dlsym(h1, "library_id");      // h1 == h2 -> silently resolves in libbeta!
 *
 * Fixtures (staged under lib/ by the per-suite RAMFS image), each exporting the
 * SAME symbol name library_id() with a DISTINCT return value so any aliasing is
 * directly observable:
 *   - libalpha.so: library_id() == 0x0A0A0A0A. Self-contained.
 *   - libbeta.so:  library_id() == 0x0B0B0B0B. Self-contained.
 *
 * Flow: load libalpha.so, confirm its symbol, then unload it (freeing its file
 * descriptor). Load libbeta.so, which the loader typically maps onto the
 * just-freed descriptor. The fresh handle must differ from the stale one, the
 * fresh handle must resolve libbeta.so's symbol, and the stale handle must fail
 * cleanly (dlsym returns NULL) instead of resolving into libbeta.so.
 *
 * The harness discards stdout in standalone terminal mode, so every check
 * returns a distinct non-zero code that pinpoints the failing step; the process
 * exits 0 only when the whole sequence succeeds.
 */

#include <dlfcn.h>
#include <stddef.h>
#include <unistd.h>

#define ALPHA_PATH "lib/libalpha.so"
#define BETA_PATH "lib/libbeta.so"

#define ALPHA_ID 0x0A0A0A0A
#define BETA_ID 0x0B0B0B0B

typedef int (*id_fn)(void);

int main(int argc, char *argv[])
{
    (void)argc;
    (void)argv;

    /* Load libalpha.so; its handle is h1. */
    void *h1 = dlopen(ALPHA_PATH, RTLD_NOW);
    if (h1 == NULL) {
        return (1);
    }

    /* Sanity: h1 resolves libalpha.so's library_id(). */
    id_fn alpha_id = NULL;
    *(void **)(&alpha_id) = dlsym(h1, "library_id");
    if (alpha_id == NULL) {
        return (2);
    }
    if (alpha_id() != ALPHA_ID) {
        return (3);
    }

    /* Unload libalpha.so; its underlying file descriptor is now free to reuse. */
    if (dlclose(h1) != 0) {
        return (4);
    }

    /* Load libbeta.so; the loader typically reuses the descriptor just freed. */
    void *h2 = dlopen(BETA_PATH, RTLD_NOW);
    if (h2 == NULL) {
        return (5);
    }

    /* The fresh handle must not alias the stale one, even on descriptor reuse. */
    if (h1 == h2) {
        return (6);
    }

    /* Sanity: h2 resolves libbeta.so's library_id(). */
    id_fn beta_id = NULL;
    *(void **)(&beta_id) = dlsym(h2, "library_id");
    if (beta_id == NULL) {
        return (7);
    }
    if (beta_id() != BETA_ID) {
        return (8);
    }

    /* The stale handle must fail cleanly, not resolve into libbeta.so. */
    if (dlsym(h1, "library_id") != NULL) {
        return (9);
    }

    /* Clean up. */
    if (dlclose(h2) != 0) {
        return (10);
    }

    /* Success. */
    (void)write(STDOUT_FILENO, "ok", 2);

    return (0);
}
