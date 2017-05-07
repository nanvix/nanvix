/*
 * Copyright(C) 2017 Davidson Francis <davidsondfgl@gmail.com>
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

/*
 * Copyright (c) 1994-2009  Red Hat, Inc. All rights reserved.
 *
 * This copyrighted material is made available to anyone wishing to use,
 * modify, copy, or redistribute it subject to the terms and conditions
 * of the BSD License.   This program is distributed in the hope that
 * it will be useful, but WITHOUT ANY WARRANTY expressed or implied,
 * including the implied warranties of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE.  A copy of this license is available at 
 * http://www.opensource.org/licenses. Any Red Hat trademarks that are
 * incorporated in the source code or documentation are not subject to
 * the BSD License and may only be used or replicated with the express
 * permission of Red Hat, Inc.
 */

/*
 * Copyright (c) 1981-2000 The Regents of the University of California.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without modification,
 * are permitted provided that the following conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright notice, 
 *       this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above copyright notice,
 *       this list of conditions and the following disclaimer in the documentation
 *       and/or other materials provided with the distribution.
 *     * Neither the name of the University nor the names of its contributors 
 *       may be used to endorse or promote products derived from this software 
 *       without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT,
 * INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 */

/*
FUNCTION
<<tzset>>---set timezone characteristics from TZ environment variable

INDEX
	tzset
INDEX
	_tzset_r

ANSI_SYNOPSIS
	#include <time.h>
	void tzset(void);
	void _tzset_r (struct _reent *);

TRAD_SYNOPSIS
	#include <time.h>
	void tzset();
	void _tzset_r (reent_ptr)
        struct _reent *reent_ptr;

DESCRIPTION
<<tzset>> examines the TZ environment variable and sets up the three
external variables: <<_timezone>>, <<_daylight>>, and <<tzname>>.  The
value of <<_timezone>> shall be the offset from the current time zone
to GMT.  The value of <<_daylight>> shall be 0 if there is no daylight
savings time for the current time zone, otherwise it will be non-zero.
The <<tzname>> array has two entries: the first is the name of the
standard time zone, the second is the name of the daylight-savings time
zone.

The TZ environment variable is expected to be in the following POSIX
format:

  stdoffset1[dst[offset2][,start[/time1],end[/time2]]]

where: std is the name of the standard time-zone (minimum 3 chars)
       offset1 is the value to add to local time to arrive at Universal time
         it has the form:  hh[:mm[:ss]]
       dst is the name of the alternate (daylight-savings) time-zone (min 3 chars)
       offset2 is the value to add to local time to arrive at Universal time
         it has the same format as the std offset
       start is the day that the alternate time-zone starts
       time1 is the optional time that the alternate time-zone starts
         (this is in local time and defaults to 02:00:00 if not specified)
       end is the day that the alternate time-zone ends
       time2 is the time that the alternate time-zone ends
         (it is in local time and defaults to 02:00:00 if not specified)

Note that there is no white-space padding between fields.  Also note that
if TZ is null, the default is Universal GMT which has no daylight-savings
time.  If TZ is empty, the default EST5EDT is used.

The function <<_tzset_r>> is identical to <<tzset>> only it is reentrant
and is used for applications that use multiple threads.

RETURNS
There is no return value.

PORTABILITY
<<tzset>> is part of the POSIX standard.

Supporting OS subroutine required: None
*/

#include <_ansi.h>
#include <reent.h>
#include <time.h>
#include "local.h"

_VOID
_DEFUN_VOID (tzset)
{
  _tzset_r (_REENT);
}
