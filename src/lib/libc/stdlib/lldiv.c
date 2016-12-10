/*
 * Copyright(C) 2016 Davidson Francis <davidsondfgl@gmail.com>
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

/*-
 * Copyright (c) 2001 Mike Barcroft <mike@FreeBSD.org>
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

#include <stdlib.h>

/*
 * The ANSI standard says that |r.quot| <= |n/d|, where
 * n/d is to be computed in infinite precision.  In other
 * words, we should always truncate the quotient towards
 * 0, never -infinity.
 *
 * Machine division and remainer may work either way when
 * one or both of n or d is negative.  If only one is
 * negative and r.quot has been truncated towards -inf,
 * r.rem will have the same sign as denom and the opposite
 * sign of num; if both are negative and r.quot has been
 * truncated towards -inf, r.rem will be positive (will
 * have the opposite sign of num).  These are considered
 * `wrong'.
 *
 * If both are num and denom are positive, r will always
 * be positive.
 *
 * This all boils down to:
 *      if num >= 0, but r.rem < 0, we got the wrong answer.
 * In that case, to get the right answer, add 1 to r.quot and
 * subtract denom from r.rem.
 */

/**
 * @brief Computes the quotient and remainder of a long division.
 *
 * @details Computes the quotient and remainder of the
 * division of the numerator @p numer by the denominator
 * @p denom. If the division is inexact, the resulting
 * quotient is the long long integer of lesser magnitude
 * that is the nearest to the algebraic quotient.
 *
 * @return Returns a structure of type lldiv_t, comprising
 * both the quotient and the remainder.
 */
lldiv_t lldiv(long long numer, long long denom)
{
	lldiv_t retval;

	retval.quot = numer / denom;
	retval.rem = numer % denom;
	if (numer >= 0 && retval.rem < 0) {
		retval.quot++;
		retval.rem -= denom;
	}
	return (retval);
}
