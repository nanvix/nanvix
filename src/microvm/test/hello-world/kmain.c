// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#include <stddef.h>
#include <stdint.h>

static void outb(uint16_t port, uint8_t value)
{
    __asm__ __volatile__("outb %0,%1"
                         : /* empty */
                         : "a"(value), "Nd"(port)
                         : "memory");
}

void kmain(void)
{
    for (const char *p = "Hello, world!\n"; *p != '\0'; p++) {
        outb(0xE9, *p);
    }
}
