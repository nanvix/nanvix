/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * libroot.so - the object the test obtains a handle to. It carries TWO
 * DT_NEEDED edges, recorded in this order by linking against the providers as
 * `-lbravo -lalpha`:
 *
 *   libroot.so
 *     +-- DT_NEEDED libbravo.so   (provider_id() == 2)  [first  in DT_NEEDED]
 *     +-- DT_NEEDED libalpha.so   (provider_id() == 1)  [second in DT_NEEDED]
 *
 * Both dependencies export the SAME symbol `provider_id()`. The DT_NEEDED order
 * (bravo, alpha) is deliberately the REVERSE of the alphabetical order of the
 * library names (libalpha.so < libbravo.so), so a lookup made through
 * libroot.so resolves to a different provider depending on the search policy:
 *
 *   * DT_NEEDED order (correct): libbravo.so  -> provider_id() == 2
 *   * alphabetical order (bug):  libalpha.so  -> provider_id() == 1
 *
 * libroot.so itself does NOT define `provider_id()`; it only references it, so
 * the loader must resolve it from the dependency list.
 */

/* Defined by BOTH dependencies. Left undefined here on purpose: the loader must
 * bind this reference to the FIRST definition in DT_NEEDED order (libbravo.so),
 * exercising the relocation-time resolution path under RTLD_NOW. */
extern int provider_id(void);

/* Unique markers, one per dependency. Referencing both forces libbravo.so and
 * libalpha.so to each be recorded as a DT_NEEDED entry (so neither is dropped by
 * an as-needed link), pinning the DT_NEEDED order to the link order. */
extern int alpha_marker(void);
extern int bravo_marker(void);

/* Touches both dependencies so both DT_NEEDED edges are present and bound. */
int root_touch(void)
{
    return alpha_marker() + bravo_marker();
}

/* Returns the id of whichever provider the loader bound libroot.so's own
 * `provider_id()` reference to. Must be libbravo.so's (2) under DT_NEEDED-order
 * resolution. */
int root_provider_id(void)
{
    return provider_id();
}
