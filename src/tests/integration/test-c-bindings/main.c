/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#include <pthread.h>
#include <sched.h>
#include <stdint.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
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

    // Assert constant macros in <stdint.h>.
    STATIC_ASSERT(INT8_C(127), INT8_MAX);
    STATIC_ASSERT(INT16_C(32767), INT16_MAX);
    STATIC_ASSERT(INT32_C(2147483647), INT32_MAX);
    STATIC_ASSERT(INT64_C(9223372036854775807), INT64_MAX);
    STATIC_ASSERT(UINT8_C(255), UINT8_MAX);
    STATIC_ASSERT(UINT16_C(65535), UINT16_MAX);
    STATIC_ASSERT(UINT32_C(4294967295), UINT32_MAX);
    STATIC_ASSERT(UINT64_C(18446744073709551615), UINT64_MAX);
    STATIC_ASSERT(INTMAX_C(9223372036854775807), INTMAX_MAX);
    STATIC_ASSERT(UINTMAX_C(18446744073709551615), UINTMAX_MAX);

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

    // Assert behavior of device-number macros in <sys/sysmacros.h>.
    STATIC_ASSERT(major(makedev(0x12, 0x34)), 0x12);
    STATIC_ASSERT(minor(makedev(0x12, 0x34)), 0x34);
    STATIC_ASSERT(major(makedev(0x1ff, 0x2ff)), 0xff);
    STATIC_ASSERT(minor(makedev(0x1ff, 0x2ff)), 0xff);

    // Assert size of types in <time.h>.
    STATIC_ASSERT_SIZE(struct timespec, sizeof(time_t) + sizeof(long));
    STATIC_ASSERT_ALIGNMENT(struct timespec, _Alignof(time_t));

    // Assert types in <sched.h>.
    STATIC_ASSERT_SIZE(struct sched_param, sizeof(int));
    STATIC_ASSERT_ALIGNMENT(struct sched_param, _Alignof(int));

    return (0);
}
