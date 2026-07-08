/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * libleaf.so - shared leaf of the dlfcn-dlclose-cycle graph.
 *
 *   libroot.so  DT_NEEDED libmidx.so, libmidy.so, libleaf.so  (direct edge)
 *   libmidx.so  DT_NEEDED libleaf.so
 *   libmidy.so  DT_NEEDED libleaf.so
 *   libleaf.so  (no dependencies)
 *
 * libleaf.so is reachable from libroot.so through THREE distinct dependency
 * edges (root->midx->leaf, root->midy->leaf, and root's own direct edge), so a
 * single dlclose(libroot.so) visits it more than once while it walks the
 * dependency graph. That repeated visit is exactly the condition that made the
 * pre-fix dlclose() panic (it removed each dependency with `extract_if` and
 * asserted `len == 1`, which tripped the second time a node was reached). The
 * post-fix reference-count peel tracks visited nodes and must instead unload
 * libleaf.so exactly once, and only after every edge that references it is gone.
 *
 * `unique_counter` is a process-lifetime counter that main.c uses as the witness
 * for a single shared instance: if the loader ever mapped libleaf.so more than
 * once, midx/midy/root would each observe their own private counter starting at
 * zero. A fresh value of zero after a full dlclose() also proves the library was
 * genuinely unloaded (its zero-initialized BSS is re-created on the next load).
 */

/* Process-lifetime counter. Bumped by every call to `leaf_bump()`. */
static int unique_counter = 0;

int leaf_bump(void)
{
    unique_counter += 1;
    return unique_counter;
}

int leaf_get(void)
{
    return unique_counter;
}
