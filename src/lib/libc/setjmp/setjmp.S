/*
 * Copyright(C) 2023 Vinicius G. Santos <viniciusgsantos@protonmail.com>
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

/* Must come first */
; #define _ASM_FILE_

; #include <i386/i386.h> ; pre-processor not working :(


.globl setjmp

.type setjmp,@function

setjmp:
    movl 4(%esp), %eax
    movl %ebx, 0(%eax)
    movl %esi, 4(%eax)
    movl %edi, 8(%eax)
    pushf
    popl 12(%eax)
    movl %ebp, 16(%eax)
    movl (%esp), %ecx
    movl %ecx, 20(%eax)
    lea 4(%esp), %ecx
    movl %ecx, 24(%eax)
    xor %eax, %eax
    ret

