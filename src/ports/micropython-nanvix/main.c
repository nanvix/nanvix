// MicroPython NanVix port — main entry point
// Runs MicroPython inside a NanVix VM.
// Usage: nanvixd.exe -- micropython.elf [-c "print('hello')"]

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "py/builtin.h"
#include "py/compile.h"
#include "py/runtime.h"
#include "py/repl.h"
#include "py/gc.h"
#include "py/mperrno.h"
#include "shared/runtime/pyexec.h"

static void do_str(const char *src, mp_parse_input_kind_t input_kind) {
    nlr_buf_t nlr;
    if (nlr_push(&nlr) == 0) {
        mp_lexer_t *lex = mp_lexer_new_from_str_len(MP_QSTR__lt_stdin_gt_, src, strlen(src), 0);
        qstr source_name = lex->source_name;
        mp_parse_tree_t parse_tree = mp_parse(lex, input_kind);
        mp_obj_t module_fun = mp_compile(&parse_tree, source_name, true);
        mp_call_function_0(module_fun);
        nlr_pop();
    } else {
        mp_obj_print_exception(&mp_plat_print, (mp_obj_t)nlr.ret_val);
    }
}

static char *stack_top;
static char heap[MICROPY_HEAP_SIZE];

int main(int argc, char **argv) {
    int stack_dummy;
    stack_top = (char *)&stack_dummy;

    gc_init(heap, heap + sizeof(heap));
    mp_init();

    // Check for -c "code" argument.
    // NanVix splits arguments on spaces (no shell-style quoting), so
    // `-c print('hello world')` arrives as argv = ["micropython", "-c", "print('hello", "world')"].
    // Rejoin everything after `-c` into a single string for MicroPython.
    if (argc >= 3 && strcmp(argv[1], "-c") == 0) {
        static char code_buf[4096];
        size_t offset = 0;
        for (int i = 2; i < argc; i++) {
            size_t arg_len = strlen(argv[i]);
            size_t space = (i > 2) ? 1 : 0;
            if (offset + space + arg_len >= sizeof(code_buf)) {
                printf("Error: -c argument too long (max %zu bytes)\n",
                       sizeof(code_buf) - 1);
                mp_deinit();
                return 1;
            }
            if (space) {
                code_buf[offset++] = ' ';
            }
            memcpy(code_buf + offset, argv[i], arg_len);
            offset += arg_len;
        }
        code_buf[offset] = '\0';
        do_str(code_buf, MP_PARSE_FILE_INPUT);
    } else if (argc >= 2) {
        // Treat argument as a script to execute via do_str.
        do_str(argv[1], MP_PARSE_FILE_INPUT);
    } else {
        // No arguments — run interactive REPL.
        pyexec_friendly_repl();
    }

    mp_deinit();
    return 0;
}

// Garbage collection: scan the stack for root pointers.
void gc_collect(void) {
    void *dummy;
    gc_collect_start();
    gc_collect_root(&dummy, ((mp_uint_t)stack_top - (mp_uint_t)&dummy) / sizeof(mp_uint_t));
    gc_collect_end();
}

// File import — not supported yet.
mp_lexer_t *mp_lexer_new_from_file(qstr filename) {
    mp_raise_OSError(MP_ENOENT);
}

mp_import_stat_t mp_import_stat(const char *path) {
    return MP_IMPORT_STAT_NO_EXIST;
}

// Fatal error handlers.
void nlr_jump_fail(void *val) {
    printf("FATAL: nlr_jump_fail\n");
    for (;;) {}
}

void NORETURN __fatal_error(const char *msg) {
    printf("FATAL: %s\n", msg);
    for (;;) {}
}

#ifndef NDEBUG
void MP_WEAK __assert_func(const char *file, int line, const char *func, const char *expr) {
    printf("Assertion '%s' failed, at file %s:%d\n", expr, file, line);
    __fatal_error("Assertion failed");
}
#endif
