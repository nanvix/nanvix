/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2015-2016 Davidson Francis <davidsondfgl@hotmail.com>
 *              2016-2016 Subhra S. Sarkar <rurtle.coder@gmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/fs.h>
#include <nanvix/hal.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <nanvix/klib.h>
#include <sys/stat.h>
#include <signal.h>
#include <limits.h>

/**
 * @brief Idle process page directory.
 */
EXTERN struct pde idle_pgdir[];

/**
 * @brief Idle process kernel stack.
 */
PUBLIC char idle_kstack[KSTACK_SIZE];

/**
 * @brief Process table.
 */
PUBLIC struct process proctab[PROC_MAX];

/**
 * @brief Current running process.
 */
PUBLIC struct process *curr_proc = IDLE;

/**
 * @brief Last running process.
 */
PUBLIC struct process *last_proc = IDLE;

/**
 * @brief Next available process ID.
 */
PUBLIC pid_t next_pid = 0;

/**
 * @brief Current number of processes in the system.
 */
PUBLIC unsigned nprocs = 0;

/**
 * @brief Initializes the process management system.
 */
PUBLIC void pm_init(void)
{
	int i;             /* Loop index.      */
	struct process *p; /* Working process. */

	/* Initialize the process table. */
	for (p = FIRST_PROC; p <= LAST_PROC; p++)
		p->flags = 0, p->state = PROC_DEAD;

	/* Handcraft init process. */
	IDLE->cr3 = (dword_t)idle_pgdir;
	IDLE->intlvl = 1;
	IDLE->flags = 0;
	IDLE->received = 0;
	IDLE->kstack = idle_kstack;
	IDLE->restorer = NULL;
	for (i = 0; i < NR_SIGNALS; i++)
		IDLE->handlers[i] = SIG_DFL;
	IDLE->irqlvl = INT_LVL_5;
	IDLE->pgdir = idle_pgdir;
	for (i = 0; i < NR_PREGIONS; i++)
		IDLE->pregs[i].reg = NULL;
	IDLE->size = 0;
	for (i = 0; i < OPEN_MAX; i++)
		IDLE->ofiles[i] = NULL;
	IDLE->close = 0;
	IDLE->umask = S_IXUSR | S_IWGRP | S_IXGRP | S_IWOTH | S_IXOTH;
	IDLE->tty = NULL_DEV;
	IDLE->status = 0;
	IDLE->nchildren = 0;
	IDLE->uid = SUPERUSER;
	IDLE->euid = SUPERUSER;
	IDLE->suid = SUPERUSER;
	IDLE->gid = SUPERGROUP;
	IDLE->egid = SUPERGROUP;
	IDLE->sgid = SUPERGROUP;
	IDLE->pid = next_pid++;
	IDLE->pgrp = IDLE;
	IDLE->father = NULL;
	kstrncpy(IDLE->name, "idle", NAME_MAX);
	IDLE->utime = 0;
	IDLE->ktime = 0;
	IDLE->cutime = 0;
	IDLE->cktime = 0;
	IDLE->state = PROC_RUNNING;
	IDLE->counter = PROC_QUANTUM;
	IDLE->priority = PRIO_USER;
	IDLE->nice = NZERO;
	IDLE->alarm = 0;
	IDLE->next = NULL;
	IDLE->chain = NULL;

	nprocs++;

	enable_interrupts();
}
