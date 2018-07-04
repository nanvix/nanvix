/*
 * Copyright(C) 2016-2018 Davidson Francis <davidsondfgl@gmail.com>
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

#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <nanvix/smp.h>
#include <i386/pmc.h>
#include <errno.h>

/*
 * Enable process accounting.
 */
PUBLIC int sys_acct(struct pmc *p, unsigned char rw)
{
	if (rw == ACCT_WR)
	{
		/* Invalid PMC. */
		if (!chkmem(p, sizeof(struct pmc), MAY_READ))
			return (-EFAULT);

		/* Check if the counters are valid. */
		if (p->enable_counters < 1 || p->enable_counters > 3)
			return (-EINVAL);

		/* Updates the kernel structure. */
		kmemcpy(&cpus[curr_core].curr_thread->pmcs, p, sizeof(struct pmc));
	}
	else if (rw == ACCT_RD)
	{
		/* Invalid PMC. */
		if (!chkmem(p, sizeof(struct pmc), MAY_WRITE))
			return (-EFAULT);

		p->enable_counters = cpus[curr_core].curr_thread->pmcs.enable_counters;
		p->event_C1 = cpus[curr_core].curr_thread->pmcs.event_C1;
		p->event_C2 = cpus[curr_core].curr_thread->pmcs.event_C2;

		if (cpus[curr_core].curr_thread->pmcs.enable_counters & 1)
			p->C1 = cpus[curr_core].curr_thread->pmcs.C1 + read_pmc(0);
		
		if (cpus[curr_core].curr_thread->pmcs.enable_counters >> 1)
			p->C2 = cpus[curr_core].curr_thread->pmcs.C2 + read_pmc(1);
	}
	else
		return (-EINVAL);

	return (0);
}
