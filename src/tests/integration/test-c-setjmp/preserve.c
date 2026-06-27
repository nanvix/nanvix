/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <setjmp.h>
#include <stdio.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests that volatile automatic variables modified after setjmp() survive a longjmp().
void test_volatile_locals_preserved(void)
{
    fprintf(stderr, "testing volatile locals across longjmp() ... ");

    jmp_buf env;
    volatile int counter = 100;
    volatile int reached = 0;

    if (setjmp(env) == 0) {
        // Mutate volatile locals after the context was saved, then jump back.
        counter += 11;
        reached = 1;
        longjmp(env, 1);
        assert(0 && "longjmp() returned");
    }

    // Resumed through longjmp(): volatile automatics keep their latest values.
    assert(reached == 1);
    assert(counter == 111);

    fprintf(stderr, "passed\n");
}

// Tests that setjmp() writes only within the jmp_buf and does not overflow into adjacent storage.
void test_jmp_buf_no_overflow(void)
{
    fprintf(stderr, "testing jmp_buf bounds ... ");

    // Sentinels placed immediately after the jmp_buf inside the same object. A
    // setjmp() implementation that writes past the end of the buffer (e.g. one
    // expecting a wider jmp_buf than the header declares) would clobber these and
    // corrupt the caller's stack. They must stay intact across both setjmp() and
    // longjmp(), which only touch the buffer itself.
    struct {
        jmp_buf env;
        volatile unsigned int sentinel[4];
    } guarded;

    guarded.sentinel[0] = 0xDEADBEEFu;
    guarded.sentinel[1] = 0xCAFEBABEu;
    guarded.sentinel[2] = 0x12345678u;
    guarded.sentinel[3] = 0xA5A5A5A5u;

    if (setjmp(guarded.env) == 0) {
        // Direct return: setjmp() must not have touched the trailing sentinels.
        assert(guarded.sentinel[0] == 0xDEADBEEFu);
        assert(guarded.sentinel[1] == 0xCAFEBABEu);
        assert(guarded.sentinel[2] == 0x12345678u);
        assert(guarded.sentinel[3] == 0xA5A5A5A5u);
        longjmp(guarded.env, 1);
        assert(0 && "longjmp() returned");
    }

    // Jump return: longjmp() only reads the buffer, so the sentinels still hold.
    assert(guarded.sentinel[0] == 0xDEADBEEFu);
    assert(guarded.sentinel[1] == 0xCAFEBABEu);
    assert(guarded.sentinel[2] == 0x12345678u);
    assert(guarded.sentinel[3] == 0xA5A5A5A5u);

    fprintf(stderr, "passed\n");
}
