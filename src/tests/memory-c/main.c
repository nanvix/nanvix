/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <unistd.h>

// TODO: remove the following line once prototype is added to unistd.h
extern void *sbrk(intptr_t increment);

//==================================================================================================
// Macros
//==================================================================================================

/**
 * @brief Performs a static assertion.
 *
 * @param a Expression to assert.
 * @param b Expected value.
 *
 * @returns Nothing. If the assertion fails, compilation will fail.
 */
#define STATIC_ASSERT(a, b) ((void)sizeof(char[(((a) == (b)) ? 1 : -1)]))

/**
 * @brief Performs a static assertion on the size of a type.
 *
 * @param a Type to assert.
 * @param b Expected size.
 *
 * @returns Nothing. If the assertion fails, compilation will fail.
 */
#define STATIC_ASSERT_SIZE(a, b) STATIC_ASSERT(sizeof(a), b)

/**
 * @brief Performs a static assertion on the alignment of a type.
 *
 * @param a Type to assert.
 * @param b Expected alignment.
 *
 * @returns Nothing. If the assertion fails, compilation will fail.
 */
#define STATIC_ASSERT_ALIGNMENT(a, b) STATIC_ASSERT(_Alignof(a), b)

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests if `sbrk()` works.
static void test_sbrk(void)
{
    // Check that `sbrk()` returns a pointer to the end of the heap.
    void *ptr = sbrk(0);
    assert(ptr != (void *)-1);

    // Check that `sbrk()` can allocate memory.
    void *new_ptr = sbrk(4096);
    assert(new_ptr != (void *)-1);
    assert(ptr == new_ptr);

    // Check that program break is where we expect it to be.
    void *new_ptr2 = sbrk(0);
    assert(new_ptr2 != (void *)-1);
    assert(new_ptr2 == (void *)((char *)ptr + 4096));

    // Check that `sbrk()` can free memory.
    void *free_ptr = sbrk(-4096);
    assert(free_ptr != (void *)-1);
    assert(free_ptr == (void *)((char *)ptr + 4096));

    // Check that program break is where we expect it to be.
    void *new_ptr3 = sbrk(0);
    assert(new_ptr3 != (void *)-1);
    assert(new_ptr3 == ptr);
}

/**
 * @brief Tests pthreads system calls.
 *
 * @param argc Number of command-line arguments (unused).
 * @param argv List of command-line arguments (unused).
 *
 * @returns Always returns zero. If a test fails, the program will abort.
 */
int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    // Test `sbrk()`.
    test_sbrk();


    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 3);
    }

    return (0);
}
