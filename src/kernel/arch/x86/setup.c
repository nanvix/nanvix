/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * This file is part of Nanvix.
 *
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <i386/i386.h>
#include <i386/int.h>
#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>

/* Descriptor tables. */
PRIVATE struct gdte gdt[GDT_SIZE]; /* Global descriptor table.    */
PRIVATE struct idte idt[IDT_SIZE]; /* Interrupt descriptor table. */

/* Descriptor tables pointers. */
PRIVATE struct gdtptr gdtptr; /* Global descriptor table pointer.    */
PRIVATE struct idtptr idtptr; /* Interrupt descriptor table pointer. */

/* Task state segment. */
PUBLIC struct tss tss;

/*
 * Sets a GDT entry.
 */
PRIVATE void set_gdte
(int n, unsigned base, unsigned limit, unsigned granularity, unsigned access)
{
    /* Set segment base address. */
    gdt[n].base_low = (base & 0xffffff);
    gdt[n].base_high = (base >> 24) & 0xff;

    /* Set segment limit. */
    gdt[n].limit_low = (limit & 0xffff);
    gdt[n].limit_high = (limit >> 16) & 0xf;

    /* Set granularity and access. */
    gdt[n].granularity = granularity;
    gdt[n].access = access;
}

/*
 * Sets up TSS.
 */
PRIVATE void tss_setup()
{
	/* Size-error checking. */
	CHKSIZE(sizeof(struct tss), TSS_SIZE);

	/* Blank TSS. */
	kmemset(&tss, 0, TSS_SIZE);

	/* Fill up TSS. */
	tss.ss0 = KERNEL_DS;
	tss.iomap = (TSS_SIZE - 1) << 16;

	/* Flush TSS. */
	tss_flush();

	kprintf("kernel: tss at %x", &tss);
}

/*
 * Sets up GDT.
 */
PRIVATE void gdt_setup(void)
{
	/* Size-error checking. */
	CHKSIZE(sizeof(struct gdte), GDTE_SIZE);
	CHKSIZE(sizeof(struct gdtptr), GDTPTR_SIZE);

	/* Blank GDT and GDT pointer. */
	kmemset(gdt, 0, sizeof(gdt));
	kmemset(&gdtptr, 0, GDTPTR_SIZE);

    /* Set GDT entries. */
    set_gdte(GDT_NULL, 0, 0x00000, 0x0, 0x00);
    set_gdte(GDT_CODE_DPL0, 0, 0xfffff, 0xc, 0x9a);
    set_gdte(GDT_DATA_DPL0, 0, 0xfffff, 0xc, 0x92);
    set_gdte(GDT_CODE_DPL3, 0, 0xfffff, 0xc, 0xfa);
    set_gdte(GDT_DATA_DPL3, 0, 0xfffff, 0xc, 0xf2);
	set_gdte(GDT_TSS, (unsigned) &tss, (unsigned)&tss + TSS_SIZE, 0x0, 0xe9);

    /* Set GDT pointer. */
    gdtptr.size = sizeof(gdt) - 1;
    gdtptr.ptr = (unsigned) gdt;

    /* Flush GDT. */
    gdt_flush(&gdtptr);
}

/*
 * Sets a IDT entry.
 */
PRIVATE void set_idte
(int n, unsigned handler, unsigned selector, unsigned flags, unsigned type)
{
	/* Set handler. */
	idt[n].handler_low = (handler & 0xffff);
    idt[n].handler_high = (handler >> 16) & 0xffff;

    /* Set GDT selector. */
    idt[n].selector = selector;

   /* Set type and flags. */
    idt[n].type = type;
    idt[n].flags = flags;
}

/*
 * Sets up IDT.
 */
PRIVATE void idt_setup(void)
{
	int i;

	/* Size-error checking. */
	CHKSIZE(sizeof(struct idte), IDTE_SIZE);
	CHKSIZE(sizeof(struct idtptr), IDTPTR_SIZE);

	/* Blank IDT and IDT pointer. */
	kmemset(idt, 0, sizeof(idt));
	kmemset(&idtptr, 0, IDTPTR_SIZE);

    /* Re-initialize PIC. */
    pic_setup(0x20, 0x28);

    /* Set software interrupts. */
    set_idte(0, (unsigned)swint0, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(1, (unsigned)swint1, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(2, (unsigned)swint2, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(3, (unsigned)swint3, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(4, (unsigned)swint4, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(5, (unsigned)swint5, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(6, (unsigned)swint6, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(7, (unsigned)swint7, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(8, (unsigned)swint8, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(9, (unsigned)swint9, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(10, (unsigned)swint10, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(11, (unsigned)swint11, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(12, (unsigned)swint12, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(13, (unsigned)swint13, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(14, (unsigned)swint14, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(15, (unsigned)swint15, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(16, (unsigned)swint16, KERNEL_CS, 0x8, IDT_INT32);
    for (i = 17; i < 32; i++)
        set_idte(i, (unsigned)swint17, KERNEL_CS, 0x8, IDT_INT32);

    /* Set hardware interrupts. */
    set_idte(32, (unsigned)hwint0, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(33, (unsigned)hwint1, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(34, (unsigned)hwint2, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(35, (unsigned)hwint3, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(36, (unsigned)hwint4, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(37, (unsigned)hwint5, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(38, (unsigned)hwint6, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(39, (unsigned)hwint7, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(40, (unsigned)hwint8, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(41, (unsigned)hwint9, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(42, (unsigned)hwint10, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(43, (unsigned)hwint11, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(44, (unsigned)hwint12, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(45, (unsigned)hwint13, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(46, (unsigned)hwint14, KERNEL_CS, 0x8, IDT_INT32);
    set_idte(47, (unsigned)hwint15, KERNEL_CS, 0x8, IDT_INT32);

    /* Set system call interrupt. */
    set_idte(128, (unsigned)syscall, KERNEL_CS, 0xe, IDT_INT32);

    /* Set IDT pointer. */
    idtptr.size = sizeof(idt) - 1;
    idtptr.ptr = (unsigned)&idt;

    /* Flush IDT. */
    idt_flush(&idtptr);
}

/*
 * Sets up machine.
 */
PUBLIC void setup(void)
{
	/* Setup descriptor tables. */
	gdt_setup();
    kprintf("boot: loading global descriptor table");
	tss_setup();
    kprintf("boot: loading interrupt descriptor table");
	idt_setup();
}
