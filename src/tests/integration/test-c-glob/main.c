/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

/* Must come first. */
#define _POSIX_C_SOURCE 200809L // AT_FDCWD, AT_REMOVEDIR

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <fcntl.h>
#include <glob.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

/// Directory under which the test fixtures are created.
#define ROOT "globdir"

//==================================================================================================
// Fixture Helpers
//==================================================================================================

// Creates an empty regular file, aborting on failure.
static void make_file(const char *path)
{
    int fd = open(path, O_CREAT | O_WRONLY | O_TRUNC, S_IRUSR | S_IWUSR);
    assert(fd != -1);
    assert(close(fd) == 0);
}

// Creates a directory, aborting on failure.
static void make_dir(const char *path)
{
    assert(mkdir(path, S_IRWXU) == 0);
}

// Removes a regular file, aborting on failure.
static void remove_file(const char *path)
{
    assert(unlink(path) == 0);
}

// Removes a directory, aborting on failure.
static void remove_dir(const char *path)
{
    assert(unlinkat(AT_FDCWD, path, AT_REMOVEDIR) == 0);
}

// Builds the fixture tree used by every test.
//
//   globdir/
//     a.txt  b.txt  c.log  file1  file2  .hidden
//     sub/   ( x.txt  y.txt )
//     sub2/  ( z.txt )
static void setup(void)
{
    make_dir(ROOT);
    make_file(ROOT "/a.txt");
    make_file(ROOT "/b.txt");
    make_file(ROOT "/c.log");
    make_file(ROOT "/file1");
    make_file(ROOT "/file2");
    make_file(ROOT "/.hidden");
    make_dir(ROOT "/sub");
    make_file(ROOT "/sub/x.txt");
    make_file(ROOT "/sub/y.txt");
    make_dir(ROOT "/sub2");
    make_file(ROOT "/sub2/z.txt");
}

// Tears the fixture tree down, in dependency order.
static void teardown(void)
{
    remove_file(ROOT "/sub/x.txt");
    remove_file(ROOT "/sub/y.txt");
    remove_dir(ROOT "/sub");
    remove_file(ROOT "/sub2/z.txt");
    remove_dir(ROOT "/sub2");
    remove_file(ROOT "/a.txt");
    remove_file(ROOT "/b.txt");
    remove_file(ROOT "/c.log");
    remove_file(ROOT "/file1");
    remove_file(ROOT "/file2");
    remove_file(ROOT "/.hidden");
    remove_dir(ROOT);
}

// Asserts that the result list holds exactly the `n` expected pathnames, in order, and that the
// list is null-terminated immediately after them (the no-offset case).
static void assert_paths(const glob_t *g, const char *const *expected, size_t n)
{
    assert(g->gl_pathc == n);
    for (size_t i = 0; i < n; i++) {
        assert(g->gl_pathv[i] != NULL);
        assert(strcmp(g->gl_pathv[i], expected[i]) == 0);
    }
    assert(g->gl_pathv[n] == NULL);
}

//==================================================================================================
// Tests
//==================================================================================================

// A pattern that matches nothing reports GLOB_NOMATCH and an empty, valid result.
static void test_no_match(void)
{
    fprintf(stderr, "testing glob() no-match ... ");

    glob_t g;
    int ret = glob(ROOT "/zzz*", 0, NULL, &g);
    assert(ret == GLOB_NOMATCH);
    assert(g.gl_pathc == 0);
    globfree(&g);

    fprintf(stderr, "passed\n");
}

// A literal pattern returns itself when the named file exists, and GLOB_NOMATCH otherwise.
static void test_literal(void)
{
    fprintf(stderr, "testing glob() literal ... ");

    glob_t g;
    const char *expected[] = {ROOT "/a.txt"};
    assert(glob(ROOT "/a.txt", 0, NULL, &g) == 0);
    assert_paths(&g, expected, 1);
    globfree(&g);

    assert(glob(ROOT "/missing.txt", 0, NULL, &g) == GLOB_NOMATCH);
    globfree(&g);

    fprintf(stderr, "passed\n");
}

// The `*` wildcard expands within a directory and the result is sorted.
static void test_star(void)
{
    fprintf(stderr, "testing glob() '*' ... ");

    glob_t g;
    const char *expected[] = {ROOT "/a.txt", ROOT "/b.txt"};
    assert(glob(ROOT "/*.txt", 0, NULL, &g) == 0);
    assert_paths(&g, expected, 2);
    globfree(&g);

    fprintf(stderr, "passed\n");
}

// The `?` wildcard matches exactly one character.
static void test_question(void)
{
    fprintf(stderr, "testing glob() '?' ... ");

    glob_t g;
    const char *expected[] = {ROOT "/file1", ROOT "/file2"};
    assert(glob(ROOT "/file?", 0, NULL, &g) == 0);
    assert_paths(&g, expected, 2);
    globfree(&g);

    fprintf(stderr, "passed\n");
}

// Bracket expressions match sets, ranges, and negations.
static void test_bracket(void)
{
    fprintf(stderr, "testing glob() '[...]' ... ");

    glob_t g;

    const char *both[] = {ROOT "/file1", ROOT "/file2"};
    assert(glob(ROOT "/file[12]", 0, NULL, &g) == 0);
    assert_paths(&g, both, 2);
    globfree(&g);

    const char *one[] = {ROOT "/file1"};
    assert(glob(ROOT "/file[1]", 0, NULL, &g) == 0);
    assert_paths(&g, one, 1);
    globfree(&g);

    const char *negated[] = {ROOT "/file2"};
    assert(glob(ROOT "/file[!1]", 0, NULL, &g) == 0);
    assert_paths(&g, negated, 1);
    globfree(&g);

    const char *range[] = {ROOT "/a.txt", ROOT "/b.txt"};
    assert(glob(ROOT "/[a-b].txt", 0, NULL, &g) == 0);
    assert_paths(&g, range, 2);
    globfree(&g);

    fprintf(stderr, "passed\n");
}

// A leading '.' is matched only by an explicit '.' in the pattern, never by a wildcard.
static void test_hidden(void)
{
    fprintf(stderr, "testing glob() leading-dot ... ");

    glob_t g;

    // '*' must skip the hidden file (and never yield "." or "..").
    const char *visible[] = {
        ROOT "/a.txt",
        ROOT "/b.txt",
        ROOT "/c.log",
        ROOT "/file1",
        ROOT "/file2",
        ROOT "/sub",
        ROOT "/sub2",
    };
    assert(glob(ROOT "/*", 0, NULL, &g) == 0);
    assert_paths(&g, visible, 7);
    globfree(&g);

    // An explicit leading '.' matches the hidden file only.
    const char *hidden[] = {ROOT "/.hidden"};
    assert(glob(ROOT "/.*", 0, NULL, &g) == 0);
    assert_paths(&g, hidden, 1);
    globfree(&g);

    fprintf(stderr, "passed\n");
}

// GLOB_MARK appends a '/' to matched directories but leaves files untouched.
static void test_mark(void)
{
    fprintf(stderr, "testing glob() GLOB_MARK ... ");

    glob_t g;

    const char *dirs[] = {ROOT "/sub/", ROOT "/sub2/"};
    assert(glob(ROOT "/sub*", GLOB_MARK, NULL, &g) == 0);
    assert_paths(&g, dirs, 2);
    globfree(&g);

    const char *files[] = {ROOT "/a.txt", ROOT "/b.txt"};
    assert(glob(ROOT "/*.txt", GLOB_MARK, NULL, &g) == 0);
    assert_paths(&g, files, 2);
    globfree(&g);

    fprintf(stderr, "passed\n");
}

// A trailing '/' restricts matches to directories and marks them with a trailing '/'.
static void test_trailing_slash(void)
{
    fprintf(stderr, "testing glob() trailing-slash ... ");

    glob_t g;
    const char *dirs[] = {ROOT "/sub/", ROOT "/sub2/"};
    assert(glob(ROOT "/*/", 0, NULL, &g) == 0);
    assert_paths(&g, dirs, 2);
    globfree(&g);

    fprintf(stderr, "passed\n");
}

// GLOB_NOCHECK returns the (verbatim) pattern when nothing matches.
static void test_nocheck(void)
{
    fprintf(stderr, "testing glob() GLOB_NOCHECK ... ");

    glob_t g;
    const char *expected[] = {ROOT "/zzz*"};
    assert(glob(ROOT "/zzz*", GLOB_NOCHECK, NULL, &g) == 0);
    assert_paths(&g, expected, 1);
    globfree(&g);

    fprintf(stderr, "passed\n");
}

// GLOB_NOSORT returns every match but in an unspecified order.
static void test_nosort(void)
{
    fprintf(stderr, "testing glob() GLOB_NOSORT ... ");

    glob_t g;
    assert(glob(ROOT "/*.txt", GLOB_NOSORT, NULL, &g) == 0);
    assert(g.gl_pathc == 2);
    assert(g.gl_pathv[2] == NULL);
    // Both names are present regardless of order.
    int seen_a = 0;
    int seen_b = 0;
    for (size_t i = 0; i < g.gl_pathc; i++) {
        if (strcmp(g.gl_pathv[i], ROOT "/a.txt") == 0) {
            seen_a = 1;
        } else if (strcmp(g.gl_pathv[i], ROOT "/b.txt") == 0) {
            seen_b = 1;
        }
    }
    assert(seen_a && seen_b);
    globfree(&g);

    fprintf(stderr, "passed\n");
}

// Patterns descend into subdirectories, both literally and through a wildcard component.
static void test_subdirectories(void)
{
    fprintf(stderr, "testing glob() subdirectories ... ");

    glob_t g;

    const char *leaves[] = {ROOT "/sub/x.txt", ROOT "/sub/y.txt"};
    assert(glob(ROOT "/sub/*.txt", 0, NULL, &g) == 0);
    assert_paths(&g, leaves, 2);
    globfree(&g);

    // An intermediate wildcard component matches only directories that actually contain the
    // trailing literal, so "sub2" (which lacks x.txt) is excluded.
    const char *cross[] = {ROOT "/sub/x.txt"};
    assert(glob(ROOT "/*/x.txt", 0, NULL, &g) == 0);
    assert_paths(&g, cross, 1);
    globfree(&g);

    fprintf(stderr, "passed\n");
}

// GLOB_APPEND concatenates a second expansion onto the first, each block sorted independently.
static void test_append(void)
{
    fprintf(stderr, "testing glob() GLOB_APPEND ... ");

    glob_t g;
    assert(glob(ROOT "/*.txt", 0, NULL, &g) == 0);
    assert(glob(ROOT "/*.log", GLOB_APPEND, NULL, &g) == 0);

    const char *expected[] = {ROOT "/a.txt", ROOT "/b.txt", ROOT "/c.log"};
    assert_paths(&g, expected, 3);
    globfree(&g);

    fprintf(stderr, "passed\n");
}

// GLOB_DOOFFS reserves leading null slots ahead of the matched pathnames.
static void test_dooffs(void)
{
    fprintf(stderr, "testing glob() GLOB_DOOFFS ... ");

    glob_t g;
    memset(&g, 0, sizeof(g));
    g.gl_offs = 2;
    assert(glob(ROOT "/*.txt", GLOB_DOOFFS, NULL, &g) == 0);
    assert(g.gl_pathc == 2);
    assert(g.gl_pathv[0] == NULL);
    assert(g.gl_pathv[1] == NULL);
    assert(strcmp(g.gl_pathv[2], ROOT "/a.txt") == 0);
    assert(strcmp(g.gl_pathv[3], ROOT "/b.txt") == 0);
    assert(g.gl_pathv[4] == NULL);
    globfree(&g);

    fprintf(stderr, "passed\n");
}

// GLOB_DOOFFS combined with GLOB_APPEND keeps the reserved slots while concatenating.
static void test_dooffs_append(void)
{
    fprintf(stderr, "testing glob() GLOB_DOOFFS|GLOB_APPEND ... ");

    glob_t g;
    memset(&g, 0, sizeof(g));
    g.gl_offs = 1;
    assert(glob(ROOT "/*.txt", GLOB_DOOFFS, NULL, &g) == 0);
    assert(glob(ROOT "/*.log", GLOB_DOOFFS | GLOB_APPEND, NULL, &g) == 0);
    assert(g.gl_pathc == 3);
    assert(g.gl_pathv[0] == NULL);
    assert(strcmp(g.gl_pathv[1], ROOT "/a.txt") == 0);
    assert(strcmp(g.gl_pathv[2], ROOT "/b.txt") == 0);
    assert(strcmp(g.gl_pathv[3], ROOT "/c.log") == 0);
    assert(g.gl_pathv[4] == NULL);
    globfree(&g);

    fprintf(stderr, "passed\n");
}

// An absolute pattern yields absolute matches.
static void test_absolute(void)
{
    fprintf(stderr, "testing glob() absolute ... ");

    char cwd[PATH_MAX];
    assert(getcwd(cwd, sizeof(cwd)) != NULL);
    // Avoid a doubled leading slash when the working directory is the root.
    const char *base = (strcmp(cwd, "/") == 0) ? "" : cwd;

    char pattern[PATH_MAX];
    char expect0[PATH_MAX];
    char expect1[PATH_MAX];
    snprintf(pattern, sizeof(pattern), "%s/" ROOT "/*.txt", base);
    snprintf(expect0, sizeof(expect0), "%s/" ROOT "/a.txt", base);
    snprintf(expect1, sizeof(expect1), "%s/" ROOT "/b.txt", base);

    glob_t g;
    assert(glob(pattern, 0, NULL, &g) == 0);
    assert(g.gl_pathc == 2);
    assert(strcmp(g.gl_pathv[0], expect0) == 0);
    assert(strcmp(g.gl_pathv[1], expect1) == 0);
    assert(g.gl_pathv[2] == NULL);
    globfree(&g);

    fprintf(stderr, "passed\n");
}

// Number of times the error callback below has been invoked.
static int errfunc_calls = 0;

// Error callback that would abort the scan (returns non-zero) and counts its invocations.
static int counting_errfunc(const char *epath, int eerrno)
{
    (void)epath;
    (void)eerrno;
    errfunc_calls++;
    return 1;
}

// A missing directory yields an empty list and is not treated as an error, even under GLOB_ERR and
// with an error callback installed (matching the POSIX "non-existing/*" rationale).
static void test_missing_directory_is_not_an_error(void)
{
    fprintf(stderr, "testing glob() missing-directory ... ");

    glob_t g;
    errfunc_calls = 0;
    int ret = glob("no_such_dir/*", GLOB_ERR, counting_errfunc, &g);
    assert(ret == GLOB_NOMATCH);
    assert(errfunc_calls == 0);
    globfree(&g);

    fprintf(stderr, "passed\n");
}

// globfree() resets the structure and is safe to call again.
static void test_globfree_resets(void)
{
    fprintf(stderr, "testing globfree() reset ... ");

    glob_t g;
    assert(glob(ROOT "/*.txt", 0, NULL, &g) == 0);
    assert(g.gl_pathc == 2);
    globfree(&g);
    assert(g.gl_pathv == NULL);
    assert(g.gl_pathc == 0);
    // A second release is a harmless no-op.
    globfree(&g);

    fprintf(stderr, "passed\n");
}

//==================================================================================================
// Entry Point
//==================================================================================================

/**
 * @brief Tests the POSIX glob()/globfree() interfaces.
 *
 * @param argc Number of command-line arguments (unused).
 * @param argv List of command-line arguments (unused).
 *
 * @returns Always returns zero. If a test fails, the program aborts.
 */
int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    setup();

    test_no_match();
    test_literal();
    test_star();
    test_question();
    test_bracket();
    test_hidden();
    test_mark();
    test_trailing_slash();
    test_nocheck();
    test_nosort();
    test_subdirectories();
    test_append();
    test_dooffs();
    test_dooffs_append();
    test_absolute();
    test_missing_directory_is_not_an_error();
    test_globfree_resets();

    teardown();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}
