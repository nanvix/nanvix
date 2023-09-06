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

.globl longjmp

.type longjmp,@function

longjmp:
    mov 4(%esp), %ecx
    mov 8(%esp), %eax 
    test %eax, %eax
    jnz valnz
    inc %eax
valnz:
    movl (%ecx), %ebx
    movl 4(%ecx), %esi
    movl 8(%ecx), %edi
    pushl 12(%ecx)
    popf
    movl 16(%ecx), %ebp
    movl 20(%ecx), %esp
    jmp *24(%ecx)
