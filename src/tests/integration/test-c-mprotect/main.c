/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * System-level test for read-only memory protection with mprotect().
 *
 * The test maps a range of anonymous pages with read and write access, fills it with a known
 * pattern, and drops write access to the whole range with mprotect(PROT_READ). It then checks that
 * the range stays readable and keeps its contents, writes the "ok" marker to stdout, and finally
 * writes to the read-only range. That write must raise a page fault, which the test deliberately
 * leaves unhandled: the process is terminated and exits with status 4 (EINTR), the status that the
 * kernel assigns to a process that is killed by a memory fault.
 *
 * The harness expects both the "ok" marker and exit status 4. The marker tells the expected
 * termination apart from a crash in an earlier step, which would exit with status 4 as well. Every
 * failed check exits with a distinct status other than 4.
 */

#include <stddef.h>
#include <sys/mman.h>
#include <unistd.h>

/* Number of pages in the protected range. */
#define NUM_PAGES 4

/* Modulus of the fill pattern: a prime that does not divide the page size, so pages differ. */
#define PATTERN_MODULUS 251

/* Exit statuses of failed checks. Status 4 is reserved for the expected termination. */
#define STATUS_BAD_PAGE_SIZE 10    /* sysconf() reported an invalid page size. */
#define STATUS_MMAP_FAILED 11      /* mmap() failed. */
#define STATUS_MPROTECT_FAILED 12  /* mprotect() failed. */
#define STATUS_CONTENTS_CHANGED 13 /* The range lost its contents after mprotect(). */
#define STATUS_NO_FAULT 14         /* Writing to the read-only range did not fault. */

/**
 * @brief Computes the byte stored at an offset of the protected range.
 *
 * @param offset Offset into the protected range.
 *
 * @returns The byte stored at @p offset.
 */
static unsigned char pattern(size_t offset)
{
    return ((unsigned char)(offset % PATTERN_MODULUS));
}

int main(int argc, char *argv[])
{
    (void)argc;
    (void)argv;

    long page_size = sysconf(_SC_PAGE_SIZE);
    if (page_size <= 0) {
        return (STATUS_BAD_PAGE_SIZE);
    }
    size_t len = (size_t)page_size * NUM_PAGES;

    /* Map a range of anonymous pages with read and write access. */
    unsigned char *range =
        mmap(NULL, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (range == MAP_FAILED) {
        return (STATUS_MMAP_FAILED);
    }

    /* Fill the range while it is writable. */
    for (size_t i = 0; i < len; i++) {
        range[i] = pattern(i);
    }

    /* Drop write access to the whole range. */
    if (mprotect(range, len, PROT_READ) != 0) {
        return (STATUS_MPROTECT_FAILED);
    }

    /* The range must stay readable and keep its contents. */
    const volatile unsigned char *readonly = range;
    for (size_t i = 0; i < len; i++) {
        if (readonly[i] != pattern(i)) {
            return (STATUS_CONTENTS_CHANGED);
        }
    }

    /* Every check that precedes the fault passed. */
    (void)write(STDOUT_FILENO, "ok", 2);

    /*
     * Write to the last page of the range, so that the fault also shows that the protection reaches
     * the end of the range. The write must terminate the process, so the code below runs only if
     * the write succeeded.
     */
    size_t offset = len - (size_t)page_size;
    volatile unsigned char *target = range + offset;
    *target = (unsigned char)~pattern(offset);

    return (STATUS_NO_FAULT);
}
