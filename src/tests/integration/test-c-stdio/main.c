/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

//==================================================================================================
// Private Functions
//==================================================================================================

// Calls vdprintf() with a freshly built variable argument list.
static int call_vdprintf(int fd, const char *format, ...)
{
    va_list args;
    int ret;

    va_start(args, format);
    ret = vdprintf(fd, format, args);
    va_end(args);

    return ret;
}

// Tests formatted output into caller-owned and dynamically allocated buffers.
static void test_formatted_buffers(void)
{
    char buffer[64];
    char *dynamic_buffer;
    int ret;

    ret = snprintf(buffer, sizeof(buffer), "int=%d float=%.1f sci=%.1e", 42, 3.5, 25.0);
    assert(ret == (int)strlen("int=42 float=3.5 sci=2.5e+01"));
    assert(strcmp(buffer, "int=42 float=3.5 sci=2.5e+01") == 0);

    dynamic_buffer = NULL;
    ret = asprintf(&dynamic_buffer, "hex=%#x text=%s value=%.2f", 255, "ok", 1.25);
    assert(ret == (int)strlen("hex=0xff text=ok value=1.25"));
    assert(dynamic_buffer != NULL);
    assert(strcmp(dynamic_buffer, "hex=0xff text=ok value=1.25") == 0);
    free(dynamic_buffer);
}

// Tests descriptor-backed formatted output and delimiter-based input.
static void test_descriptor_and_line_io(void)
{
    const char *filename = "stdio-c.tmp";
    FILE *stream;
    char *line;
    size_t capacity;
    ssize_t length;
    int fd;
    int ret;

    stream = fopen(filename, "w+");
    assert(stream != NULL);

    fd = fileno(stream);
    assert(fd >= 0);

    ret = dprintf(fd, "alpha %d %.1f\n", 7, 2.5);
    assert(ret == (int)strlen("alpha 7 2.5\n"));

    ret = call_vdprintf(fd, "beta %s!", "done");
    assert(ret == (int)strlen("beta done!"));

    assert(fseeko(stream, 0, SEEK_SET) == 0);

    line = NULL;
    capacity = 0;
    length = getline(&line, &capacity, stream);
    assert(length == (ssize_t)strlen("alpha 7 2.5\n"));
    assert(strcmp(line, "alpha 7 2.5\n") == 0);

    length = getdelim(&line, &capacity, '!', stream);
    assert(length == (ssize_t)strlen("beta done!"));
    assert(strcmp(line, "beta done!") == 0);

    free(line);
    assert(fclose(stream) == 0);
    assert(remove(filename) == 0);
}

// Tests reopening an existing FILE object on a new pathname.
static void test_freopen(void)
{
    const char *first = "stdio-c-freopen-a.tmp";
    const char *second = "stdio-c-freopen-b.tmp";
    FILE *stream;
    char buffer[16];

    stream = fopen(first, "w+");
    assert(stream != NULL);
    assert(fputs("old\n", stream) >= 0);

    assert(freopen(second, "w+", stream) == stream);
    assert(fputs("new\n", stream) >= 0);
    assert(fseeko(stream, 0, SEEK_SET) == 0);
    assert(fgets(buffer, sizeof(buffer), stream) == buffer);
    assert(strcmp(buffer, "new\n") == 0);

    assert(fclose(stream) == 0);
    assert(remove(first) == 0);
    assert(remove(second) == 0);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/**
 * @brief Tests standard I/O calls exposed by <stdio.h>.
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

    test_formatted_buffers();
    test_descriptor_and_line_io();
    test_freopen();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}
