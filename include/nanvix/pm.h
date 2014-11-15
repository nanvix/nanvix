/*
 * Copyright(C) 2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

/*
 * File: nanvix/pm.h
 * 
 * Process management system library.
 * 
 * The process management system library provides the abstraction of processes
 * and several function for dealing with them.
 */

#ifndef PM_H_
#define PM_H_

	#include <i386/i386.h>
	#include <nanvix/config.h>
	#include <nanvix/const.h>
	#include <nanvix/fs.h>
	#include <nanvix/region.h>
	#include <sys/types.h>
	#include <limits.h>
	#include <signal.h>
	
	/*
	 * Constants: Superuser Credentials
	 * 
	 * Credentials used for superuser authentication.
	 * 
	 * SUPERUSER  - Superuser ID.
	 * SUPERGROUP - Superuser group ID.
	 */
	#define SUPERUSER  0
	#define SUPERGROUP 0

	/*
	 * Constants: Important System Processes
	 * 
	 * Processes that are vital to the system.
	 * 
	 * IDLE - idle process.
	 * INIT - init process.
	 */
	#define IDLE (&proctab[0])
	#define INIT (&proctab[1])
	
	/*
	 * Constants: Process Table Boundaries
	 * 
	 * Lower and upper process table boundaries.
	 * 
	 * FIRST_PROC - First process in the process table slot.
	 * LAST_PROC  - Last process in the process table slot.
	 */
	#define FIRST_PROC ((&proctab[1]))
	#define LAST_PROC ((&proctab[PROC_MAX - 1]))
	
	/*
	 * Constants: Process Flags
	 * 
	 * Process flags.
	 * 
	 * PROC_FREE   - Is the process free?
	 * PROC_NEW    - Is the process new?
	 * PROC_JMPSET - Process long jump set.
	 */
	#define PROC_FREE   (1 << 0)
	#define PROC_NEW    (1 << 1)
	#define PROC_JMPSET (1 << 2)
	
	/*
	 * Constants: Process Parameters
	 * 
	 * Parameter of the <process> structure.
	 * 
	 * PROC_QUANTUM - Quantum.
	 * NR_PREGIONS  - Number of memory regions. 
	 */
	#define PROC_QUANTUM 50
	#define NR_PREGIONS   4
	
	/*
	 * Constants: Process Priorities
	 * 
	 * Priorities of a process.
	 * 
	 * PRIO_IO         - Waiting for block operation.
	 * PRIO_BUFFER     - Waiting for buffer.
	 * PRIO_INODE      - Waiting for inode.
	 * PRIO_SUPERBLOCK - Waiting for super block.
	 * PRIO_TTY        - Waiting for terminal I/O.
	 * PRIO_REGION     - Waiting for memory region.
	 * PRIO_SIG        - Waiting for signal.
	 * PRIO_USER       - User priority.
	 */
	#define PRIO_IO         -120
	#define PRIO_BUFFER     -100
	#define PRIO_INODE       -80
	#define PRIO_SUPERBLOCK  -60
	#define PRIO_TTY         -40
	#define PRIO_REGION      -20
	#define PRIO_SIG           0
	#define PRIO_USER         20

	/*
	 * Constants: Process States
	 * 
	 * States of a process.
	 * 
	 * PROC_DEAD     - Dead.
	 * PROC_ZOMBIE   - Zombie.
	 * PROC_RUNNING  - Running.
	 * PROC_READY    - Ready to execute.
	 * PROC_WAITING  - Waiting (interruptible).
	 * PROC_SLEEPING - Waiting (uninterruptible).
	 * PROC_STOPPED  - Stopped.
	 */
	#define PROC_DEAD     0
	#define PROC_ZOMBIE   1
	#define PROC_RUNNING  2
	#define PROC_READY    3
	#define PROC_WAITING  4
	#define PROC_SLEEPING 5
	#define PROC_STOPPED  6
	
	/*
	 * Constants: Offsets to Hard-Coded Fields of a Process
	 * 
	 * Offset to hard-coded fields of a <process>.
	 * 
	 * PROC_KESP      - Kernel stack pointer offset.
	 * PROC_CR3       - Page directory pointer offset.
	 * PROC_INTLVL    - Interrupt level offset.
	 * PROC_FLAGS     - <Process flags> offset.
	 * PROC_RECEIVED  - Received signals offset.
	 * PROC_KSTACK    - Kernel stack pointer offset.
	 * PROC_HANDLERS  - Sginal handlers offset.
	 */
	#define PROC_KESP        0
	#define PROC_CR3         4
	#define PROC_INTLVL      8
	#define PROC_FLAGS      12
	#define PROC_RECEIVED   16
	#define PROC_KSTACK     20
	#define PROC_HANDLERS   24

#ifndef _ASM_FILE_

	/*
	 * Structure: process
	 * 
	 * Process.
	 * 
	 * Description:
	 * 
	 *     A process is an abstraction of a program in execution, it makes 
	 *     easier to deal with multiple parallel activities, being a core
	 *     concept of every operating system.
	 */
	struct process
	{
		/*
		 * Variables: Hard-coded fields
		 * 
		 * Hard-coded fields referenced by assembly code.
		 * 
		 * kesp     - Kernel stack pointer.
		 * cr3      - Page directory pointer.
		 * intlvl   - Interrupt level.
		 * flags    - <Process Flags>.
		 * received - Received signals.
		 * kstack   - Kernel stack pointer.
		 * handlers - Signal handlers.
		 */
    	dword_t kesp;
    	dword_t cr3;
		dword_t intlvl;
		unsigned flags;
    	unsigned received;
    	void *kstack;
		sighandler_t handlers[NR_SIGNALS];
		
    	/*
    	 * Variables: Memory information
    	 * 
    	 * Memory system information.
    	 * 
    	 * pgdir - Page directory.
    	 * pregs - Process memory regions.
    	 * size  - Process size.
    	 * kenv  - Environment for klongjmp().
    	 */
		struct pde *pgdir;
		struct pregion pregs[NR_PREGIONS];
		size_t size;
		kjmp_buf kenv;
		
		/*
		 * Variables: File system information
		 * 
		 * File system information
		 * 
		 * pwd    - Working directory.
		 * root   - Root directory.
		 * ofiles - Opened files.
		 * close  - Close on <exec>?
		 * umask  - User file's creation mask.
		 * tty    - Associated tty device.
		 */
		struct inode *pwd;
		struct inode *root;
		struct file *ofiles[OPEN_MAX];
		int close;
		mode_t umask;
		dev_t tty;
		
		/*
		 * Variables: General information
		 * 
		 * General information.
		 * 
		 * status    - Exit status.
		 * errno     - Error code.
		 * nchildren - Number of children.
		 * uid       - User ID.
		 * euid      - Effective user ID.
		 * suid      - Saved set-user-ID.
		 * gid       - Group ID.
		 * egid      - Effective group user ID.
		 * sgid      - Saved set-group-ID.
		 * pid       - Process ID.
		 * pgrp      - Process group ID.
		 * father    - Father process.
		 */
		int status;
		int errno;
		unsigned nchildren;
		uid_t uid;
		uid_t euid;
		uid_t suid;
		gid_t gid;
		gid_t egid;
		gid_t sgid;
    	pid_t pid;
    	struct process *pgrp;
    	struct process *father;
    	
    	/*
    	 * Variables: Timing information
    	 * 
    	 * Timing information.
    	 * 
    	 * utime  - User CPU time.
    	 * ktime  - Kernel CPU time.
    	 * cutime - User CPU time of terminated child processes.
    	 * cktime - Kernel CPU time of terminated child processes.
    	 */
    	unsigned utime;
    	unsigned ktime;
		unsigned cutime;
		unsigned cktime;

    	/*
    	 * Variables: Scheduling information
    	 * 
    	 * Scheduling information.
    	 * 
    	 * state    - Current state.
    	 * counter  - Remaining quantum.
    	 * priority - <Process Priorities>.
    	 * nice     - Nice for scheduling.
    	 * alarm    - Alarm.
    	 * next     - Next process in a list.
    	 * chain    - Sleeping chain.
    	 */
    	unsigned state;
    	int counter;
    	int priority;
    	int nice;
    	unsigned alarm;
		struct process *next;
		struct process **chain;
	};

/*============================================================================*
 * Section: Process State Control                                             *
 *============================================================================*/
	
	/*
	 * Function: bury
	 * 
	 * Buries a zombie process.
	 * 
	 * Parameters:
	 * 
	 *     proc - Process to bury.
	 * 
	 * Description:
	 * 
	 *     The <bury> function releases all remaining resources that are
	 *     associated to the process pointed to by _proc_ that could not be
	 *     released before, such as the page table.
	 * 
	 * See Also:
	 * 
	 *     <die>
	 */
	EXTERN void bury(struct process *proc);
	
	/*
	 * Function: die
	 * 
	 * Kills the current running process.
	 * 
	 * Parameters:
	 * 
	 *     status - Exit status code.
	 * 
	 * Description:
	 * 
	 *     The <die> function kills the current running process, releasing most
	 *     of the resources that are associated to it, such as opened files and
	 *     memory regions. 
	 * 
	 *     The exit _status_ code is saved in the process table entry so that
	 *     it can be retrieved later.
	 * 
	 * Notes:
	 * 
	 *     - Upon completion, a new process is chosen to be executed.
	 *     - Resources that could not be freed can be deallocated later by
	 *       calling the <bury> function.
	 * 
	 * See Also:
	 * 
	 *     <bury>, <yield>
	 */
	EXTERN void die(int status);
	
	/*
	 * Function: sched
	 * 
	 * Schedules a process to execution.
	 * 
	 * Parameters:
	 * 
	 *     proc - Process to be schedule.
	 * 
	 * Description:
	 * 
	 *     The <sched> function schedules the process pointed to by _process_
	 *     to later execution by effectively inserting it in the ready processes
	 *     queue.
	 */
	EXTERN void sched(struct process *proc);
	
	/*
	 * Function: sleep
	 * 
	 * Puts the current process to sleep in a chain.
	 * 
	 * Parameters:
	 * 
	 *     chain    - Target chain.
	 *     priority - Priority level.
	 * 
	 * Description:
	 * 
	 *     The <sleep> function puts the current running process to sleep in
	 *     the _chain_ of processes using the priority level _priority_.
	 * 
	 * Notes:
	 * 
	 *    - The calling process will block until it is awaken by another
	 *      process.
	 *    - Upon completion, a new process is chosen to be executed.
	 * 
	 * See Also:
	 * 
	 *    <wakeup>, <yield>
	 */
	EXTERN void sleep(struct process **chain, int priority);

	/*
	 * Function: wakeup
	 * 
	 * Wakes up all processes that are sleeping in a chain.
	 * 
	 * Parameters:
	 * 
	 *     chain - Target chain.
	 * 
	 * Description:
	 * 
	 *     The <wakeup> function wakes up all processes that are sleeping in the
	 *     chain of processes pointed to by _chain_.
	 * 
	 * See Also:
	 * 
	 *     <sleep>
	 */
	EXTERN void wakeup(struct process **chain);
	
	/*
	 * Function: yield
	 * 
	 * Yields the processor.
	 * 
	 * Description:
	 * 
	 *     The <yield> function causes the calling process to release the
	 *     processor and choose another process to execute.
	 * 
	 * Notes:
	 * 
	 *     - This function must be called in an interrupt-safe environment.
	 *     - Upon completion, a new process is chosen to be executed.
	 */
	EXTERN void yield(void);

/*============================================================================*
 * Section: Signal Handling                                                   *
 *============================================================================*/

	/*
	 * Function: sndsig
	 * 
	 * Sends a signal to a process.
	 * 
	 * Parameters:
	 * 
	 *     proc - Target process.
	 *     sing - Signal.
	 * 
	 * Description:
	 * 
	 *     The <sndsig> function sends the signal _sig_ to the process pointed
	 *     to by _proc_.
	 * 
	 * See Also:
	 * 
	 *     <issig>
	 */
	EXTERN void sndsig(struct process *proc, int sig);
	
	/*
	 * Function: issig
	 * 
	 * Checks if the current process has a pending signal.
	 * 
	 * Description:
	 * 
	 *     The <issig> function checks if the current process has a pending
	 *     signal or not.
	 * 
	 * Return:
	 * 
	 *    If there is no pending signal, <SIGNULL> is returned. However, if
	 *    there is a pending signal, that signal is returned instead.
	 */
	EXTERN int issig(void);
	
/*============================================================================*
 * Section: Miscellaneous                                                     *
 *============================================================================*/
	
	/*
	 * Function: pm_init
	 * 
	 * Initializes the process management system.
	 * 
	 * Description:
	 * 
	 *     The <pm_init> function initializes the process management system by,
	 *     ultimately initializing the process table and hand-crafting the 
	 *     <IDLE> process.
	 */
	EXTERN void pm_init(void);
	
	/*
	 * Macros: Process memory regions
	 * 
	 * Returns the requested memory region of a process.
	 * 
	 * TEXT  - Text region.
	 * DATA  - Data region.
	 * STACK - Stack region.
	 * HEAP  - Heap region.
	 */
	#define TEXT(p)  (&p->pregs[0])
	#define DATA(p)  (&p->pregs[1])
	#define STACK(p) (&p->pregs[2])
	#define HEAP(p)  (&p->pregs[3])
	
	/*
	 * Macro: KERNEL_RUNNING
	 * 
	 * Asserts if a process is running in kernel mode.
	 */
	#define KERNEL_RUNNING(p) ((p)->intlvl > 1)
	
	/*
	 * Macro: IS_LEADER
	 * 
	 * Asserts if a process is the session leader.
	 */
	#define IS_LEADER(p) ((p)->pgrp->pid == (p)->pid)
	
	/*
	 * Macro: IS_SUPERUSER
	 * 
	 * Asserts if a process has superuser privileges.
	 */
	#define IS_SUPERUSER(p) \
		(((p)->uid == SUPERUSER) || ((p)->euid == SUPERUSER))	
		
	/*
	 * Variable: proctab
	 * 
	 * Process table.
	 */
	EXTERN struct process proctab[PROC_MAX];
	
	/*
	 * Variable: curr_proc
	 * 
	 * Current running process.
	 */
	EXTERN struct process *curr_proc;
	
	/*
	 * Variable: next_pid
	 * 
	 * Next available PID.
	 */
	EXTERN pid_t next_pid;
	
	/*
	 * Variable: nprocs
	 * 
	 * Current number of processes in the system.
	 */
	EXTERN unsigned nprocs;

#endif /* _ASM_FILE */

#endif /* PM_H_ */
