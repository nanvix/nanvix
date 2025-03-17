/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#include <pthread.h>
#include <sched.h>
#include <stdint.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <time.h>

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
// Main Function
//==================================================================================================

/**
 * @brief Performs static assertions on C bindings.
 *
 * @param argc Number of command-line arguments (unused).
 * @param argv List of command-line arguments (unused).
 *
 * @returns Always returns zero. This function performs static assertions on the
 * C bindings. If any assertion fails, compilation will fail.
 */
int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    // Assert size of signed primitive types.
    STATIC_ASSERT_SIZE(char, 1);
    STATIC_ASSERT_SIZE(short, 2);
    STATIC_ASSERT_SIZE(int, 4);
    STATIC_ASSERT_SIZE(long, 4);
    STATIC_ASSERT_SIZE(long long, 8);
    STATIC_ASSERT_SIZE(float, 4);
    STATIC_ASSERT_SIZE(double, 8);

    // Assert size of unsigned primitive types.
    STATIC_ASSERT_SIZE(unsigned char, 1);
    STATIC_ASSERT_SIZE(unsigned short, 2);
    STATIC_ASSERT_SIZE(unsigned int, 4);
    STATIC_ASSERT_SIZE(unsigned long, 4);
    STATIC_ASSERT_SIZE(unsigned long long, 8);

    // Assert size of types in <stdint.h>.
    STATIC_ASSERT_SIZE(int8_t, 1);
    STATIC_ASSERT_SIZE(int16_t, 2);
    STATIC_ASSERT_SIZE(int32_t, 4);
    STATIC_ASSERT_SIZE(int64_t, 8);
    STATIC_ASSERT_SIZE(uint8_t, 1);
    STATIC_ASSERT_SIZE(uint16_t, 2);
    STATIC_ASSERT_SIZE(uint32_t, 4);
    STATIC_ASSERT_SIZE(uint64_t, 8);

    // Assert size of types int <sys/types.h>.
    STATIC_ASSERT_SIZE(blkcnt_t, sizeof(long long));
    STATIC_ASSERT_SIZE(blksize_t, sizeof(long long));
    STATIC_ASSERT_SIZE(clock_t, (sizeof(long long)));
    STATIC_ASSERT_SIZE(clockid_t, sizeof(int));
    STATIC_ASSERT_SIZE(dev_t, sizeof(unsigned long long));
    STATIC_ASSERT_SIZE(gid_t, sizeof(unsigned int));
    STATIC_ASSERT_SIZE(ino_t, sizeof(unsigned long long));
    STATIC_ASSERT_SIZE(mode_t, sizeof(unsigned int));
    STATIC_ASSERT_SIZE(nlink_t, sizeof(unsigned long long));
    STATIC_ASSERT_SIZE(off_t, sizeof(long long));
    STATIC_ASSERT_SIZE(pid_t, sizeof(int));
    STATIC_ASSERT_SIZE(reclen_t, sizeof(unsigned short));
    STATIC_ASSERT_SIZE(size_t, sizeof(unsigned int));
    STATIC_ASSERT_SIZE(ssize_t, sizeof(int));
    STATIC_ASSERT_SIZE(time_t, sizeof(long long));
    STATIC_ASSERT_SIZE(uid_t, sizeof(unsigned int));

    // Assert size of types in <time.h>.
    STATIC_ASSERT_SIZE(struct timespec, sizeof(time_t) + sizeof(long));

    // Assert types in <sched.h>.A
    STATIC_ASSERT_SIZE(struct sched_param, sizeof(int));

    return (0);
}
