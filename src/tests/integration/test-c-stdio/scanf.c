/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#include <assert.h>
#include <errno.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

static int call_vfscanf(FILE *stream, const char *format, ...)
{
    va_list args;
    va_start(args, format);
    int ret = vfscanf(stream, format, args);
    va_end(args);
    return ret;
}

static int call_vscanf(const char *format, ...)
{
    va_list args;
    va_start(args, format);
    int ret = vscanf(format, args);
    va_end(args);
    return ret;
}

static FILE *input_file(const char *text)
{
    FILE *stream = fopen("stdio-scan.tmp", "w+");
    assert(stream != NULL);
    assert(fputs(text, stream) >= 0);
    rewind(stream);
    return stream;
}

static void test_file_scanners(void)
{
    FILE *stream = input_file(" -42 017 0x2a 99 word xyz!ab]9");
    int decimal = 0, octal = 0, consumed = -1;
    unsigned int hex = 0;
    char word[5], chars[3], set[5];
    assert(fscanf(stream, "%d %i %x %*u %4s %3c%n", &decimal, &octal, &hex, word, chars,
                  &consumed) == 5);
    assert(decimal == -42 && octal == 15 && hex == 42);
    assert(strcmp(word, "word") == 0 && memcmp(chars, "xyz", 3) == 0);
    assert(consumed == (int)strlen(" -42 017 0x2a 99 word xyz"));
    assert(ftell(stream) == consumed);
    assert(ftello(stream) == consumed);
    assert(fgetc(stream) == '!');
    assert(call_vfscanf(stream, "%4[]a-b]", set) == 1);
    assert(strcmp(set, "ab]") == 0);
    assert(fgetc(stream) == '9');
    assert(fscanf(stream, "%d", &decimal) == EOF);
    assert(feof(stream) && !ferror(stream));
    assert(fclose(stream) == 0);

    stream = input_file("1234,tail\n");
    assert(fscanf(stream, "%2d", &decimal) == 1 && decimal == 12);
    assert(fgetc(stream) == '3');
    assert(call_vfscanf(stream, "%d,%4[^\n]", &decimal, set) == 2);
    assert(decimal == 4 && strcmp(set, "tail") == 0);
    assert(fgetc(stream) == '\n');
    assert(fclose(stream) == 0);

    stream = input_file("-42!");
    long wide = 0, count = -1;
    assert(fscanf(stream, "%ld%ln", &wide, &count) == 1);
    assert(wide == -42 && count == 3);
    assert(fgetc(stream) == '!');
    assert(fclose(stream) == 0);
}

static void test_scan_failures(void)
{
    int value = 17, consumed = -1;
    FILE *stream = input_file("x");
    assert(fscanf(stream, "%d", &value) == 0 && value == 17);
    assert(!feof(stream) && !ferror(stream));
    assert(fgetc(stream) == 'x');
    assert(call_vfscanf(stream, "%n%d", &consumed, &value) == 0 && consumed == 0);
    assert(feof(stream));
    assert(fclose(stream) == 0);

    stream = input_file("7");
    assert(fscanf(stream, "%*d%d", &value) == 0);
    assert(feof(stream));
    assert(fclose(stream) == 0);

    stream = input_file("+");
    assert(fscanf(stream, "%d", &value) == 0);
    assert(feof(stream));
    assert(fclose(stream) == 0);

    stream = input_file("0x!");
    assert(fscanf(stream, "%i", &value) == 0);
    assert(fgetc(stream) == '!');
    assert(fclose(stream) == 0);

    stream = input_file("  ");
    assert(fscanf(stream, "%d", &value) == EOF);
    assert(feof(stream) && !ferror(stream));
    assert(fclose(stream) == 0);

    stream = input_file("");
    assert(close(fileno(stream)) == 0);
    assert(fscanf(stream, "%d", &value) == EOF);
    assert(ferror(stream) && !feof(stream));
    assert(fclose(stream) == EOF);
}

static void test_scan_eof_indicator(void)
{
    FILE *stream = input_file("1");
    int value = 0;
    assert(fscanf(stream, "%d", &value) == 1 && value == 1 && feof(stream));
    int writer = dup(fileno(stream));
    assert(writer >= 0);
    /* Append through the descriptor without clearing the stream's EOF indicator. */
    assert(write(writer, " 2", 2) == 2);
    assert(lseek(writer, 1, SEEK_SET) == 1);
    assert(fscanf(stream, "%d", &value) == EOF && value == 1);
    assert(feof(stream) && !ferror(stream));
    clearerr(stream);
    assert(fscanf(stream, "%d", &value) == 1 && value == 2 && feof(stream));
    assert(write(writer, " 3", 2) == 2);
    assert(lseek(writer, 3, SEEK_SET) == 3);
    assert(fscanf(stream, "%d", &value) == EOF && value == 2);
    assert(fseek(stream, 0, SEEK_CUR) == 0 && !feof(stream));
    assert(fscanf(stream, "%d", &value) == 1 && value == 3 && feof(stream));
    assert(ungetc('4', stream) == '4' && !feof(stream));
    assert(fscanf(stream, "%d", &value) == 1 && value == 4 && feof(stream));
    assert(close(writer) == 0);
    assert(fclose(stream) == 0);
}

static void test_scan_relative_seek(void)
{
    FILE *stream = input_file("12,34!");
    int value = 0;
    assert(fscanf(stream, "%d", &value) == 1 && value == 12);
    assert(fseek(stream, 0, SEEK_CUR) == 0);
    assert(fgetc(stream) == ',');
    assert(fscanf(stream, "%d", &value) == 1 && value == 34);
    assert(fseeko(stream, 0, SEEK_CUR) == 0);
    assert(fgetc(stream) == '!');
    rewind(stream);
    assert(fscanf(stream, "%d", &value) == 1);
    assert(fseeko(stream, 1, SEEK_CUR) == 0);
    assert(fgetc(stream) == '3');
    assert(fseek(stream, -1, SEEK_CUR) == 0);
    assert(fgetc(stream) == '3');
    rewind(stream);
    assert(fscanf(stream, "%d", &value) == 1);
    errno = 0;
    assert(fseeko(stream, INT64_MIN, SEEK_CUR) == -1 && errno == EOVERFLOW);
    assert(ferror(stream));
    assert(fgetc(stream) == ',');
    assert(fclose(stream) == 0);
}

static void test_stdin_scanners(void)
{
    FILE *stream = input_file("12 34!");
    int saved = dup(STDIN_FILENO);
    assert(saved >= 0);
    assert(dup2(fileno(stream), STDIN_FILENO) == STDIN_FILENO);
    clearerr(stdin);
    int value = 0;
    assert(scanf("%d", &value) == 1 && value == 12);
    assert(call_vscanf(" %d", &value) == 1 && value == 34);
    assert(fgetc(stdin) == '!');
    assert(scanf("%d", &value) == EOF && feof(stdin));
    assert(dup2(saved, STDIN_FILENO) == STDIN_FILENO);
    assert(close(saved) == 0);
    clearerr(stdin);
    assert(fclose(stream) == 0);
}

static void test_scan_bytes_and_pipe(void)
{
    int fds[2];
    assert(pipe(fds) == 0);
    /* Keep the writer open: a width-limited conversion must not read ahead. */
    assert(write(fds[1], "42", 2) == 2);
    FILE *stream = fdopen(fds[0], "r");
    assert(stream != NULL);
    int value = 0;
    assert(fscanf(stream, "%2d", &value) == 1 && value == 42);
    assert(write(fds[1], "\0!", 2) == 2);
    char byte = 1;
    assert(fscanf(stream, "%c", &byte) == 1 && byte == '\0');
    assert(fgetc(stream) == '!');
    assert(close(fds[1]) == 0);
    assert(fclose(stream) == 0);
}

void test_scanners(void)
{
    test_file_scanners();
    test_scan_failures();
    test_scan_eof_indicator();
    test_scan_relative_seek();
    test_stdin_scanners();
    test_scan_bytes_and_pipe();
    assert(remove("stdio-scan.tmp") == 0);
}
