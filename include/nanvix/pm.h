/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * pm.h - Process management
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
	
	/* Process quantum. */
	#define PROC_QUANTUM 50

	/* Process priorities. */
	#define PRIO_IO         -120 /* Waiting for block operation. */
	#define PRIO_BUFFER     -100 /* Waiting for buffer.          */
	#define PRIO_INODE      -80  /* Waiting for inode.           */
	#define PRIO_SUPERBLOCK -60  /* Waiting for super block.     */
	#define PRIO_TTY        -40  /* Waiting for terminal I/O.    */
	#define PRIO_REGION     -20  /* Waiting for memory region.   */
	#define PRIO_SIG          0  /* Waiting for signal.          */
	#define PRIO_USER        20  /* User priority.               */
	
	/* Process flags. */
	#define PROC_FREE   1 /* Process is free.       */
	#define PROC_NEW    2 /* Process is new.        */
	#define PROC_JMPSET 4 /* Process long jump set. */

	/* Process states. */
	#define PROC_DEAD     0 /* Dead.                      */
	#define PROC_ZOMBIE   1 /* Zombie.                    */
	#define PROC_RUNNING  2 /* Running.       	          */
	#define PROC_READY    3 /* Ready to execute.          */
	#define PROC_WAITING  4 /* Waiting (interruptible).   */
	#define PROC_SLEEPING 5 /* Waiting (uninterruptible). */
	#define PROC_STOPPED  6 /* Stopped.                   */

	/* Offsets to the process structure. */
	#define PROC_KESP        0
	#define PROC_CR3         4
	#define PROC_INTLVL      8
	#define PROC_FLAGS      12
	#define PROC_RECEIVED   16
	#define PROC_KSTACK     20
	#define PROC_HANDLERS   24
	#define PROC_RESTORERS 116
	
	/* Superuser ID. */
	#define SUPERUSER 0
	
	/* Superuser group ID. */
	#define SUPERGROUP 0
	
	/* Process regions. */
	#define TEXT(p)  (&p->pregs[0]) /* Text region.  */
	#define DATA(p)  (&p->pregs[1]) /* Data region.  */
	#define STACK(p) (&p->pregs[2]) /* Stack region. */
	#define HEAP(p)  (&p->pregs[3]) /* Heap region.  */
	
	/* Number of process memory regions. */
	#define NR_PREGIONS 4

#ifndef _ASM_FILE_

	/*
	 * Process.
	 */
	struct process
	{
		/* Hardcoded fields. */
    	dword_t kesp;                        /* Kernel stack poiner.       */
    	dword_t cr3;                         /* Page directory pointer.    */
		dword_t intlvl;                      /* Interrupt level.           */
		int flags;                           /* Process flags (see above). */
    	int received;                        /* Received signals.          */
    	void *kstack;                        /* Kernel stack.              */
		sighandler_t handlers[NR_SIGNALS];   /* Signal handlers.           */
		
    	/* Memory information. */
		struct pde *pgdir;                 /* Page directory.             */
		struct pregion pregs[NR_PREGIONS]; /* Process memory regions.     */
		size_t size;                       /* Process size.               */
		kjmp_buf kenv;                     /* Environment for klongjmp(). */
		
		/* File system information. */
		struct inode *pwd;             /* Working directory.          */
		struct inode *root;            /* Root directory.             */
		struct file *ofiles[OPEN_MAX]; /* Open files.                 */
		int close;                     /* File to be close on exec(). */
		mode_t umask;                  /* User file's creation mask.  */
		dev_t tty;                     /* Associated tty device.      */
		
		/* General information. */
		int status;             /* Exit status.         */
		int errno;              /* Error code.          */
		int nchildren;          /* Number of children.  */
		uid_t uid;              /* User ID.             */
		uid_t euid;             /* Efective user ID.    */
		uid_t suid;             /* Saved set-user-ID.   */
		gid_t gid;              /* Group ID.            */
		gid_t egid;             /* Efective user ID.    */
		gid_t sgid;             /* Saved set-group-ID.  */
    	pid_t pid;              /* Process ID.          */
    	struct process *pgrp;   /* Process group ID.    */
    	struct process *father; /* Father process.      */
    	
    	/* Timing information. */
    	int utime;  /* User CPU time.                                 */
    	int ktime;  /* Kernel CPU time.                               */
		int cutime; /* User CPU time of terminated child processes.   */
		int cktime; /* Kernel CPU time of terminated child processes. */

    	/* Scheduling information. */
    	int state;              /* Current state.          */
    	int counter;            /* Remaining quantum.      */
    	int priority;           /* Priority.               */
    	int nice;               /* Nice for scheduling.    */
    	unsigned alarm;         /* Alarm.                  */
		struct process *next;   /* Next process in a list. */
		struct process **chain; /* Sleeping chain.         */
	};
	
	EXTERN void pm_init();
	
	EXTERN void sleep(struct process **chain, int priority);
	
	EXTERN void wakeup(struct process **chain);

	EXTERN void terminate(int err);

	EXTERN void stop();	
	
	EXTERN void resume(struct process *proc);
	
	EXTERN void abort(int err);
	
	EXTERN void die(int die);
	
	EXTERN void bury(struct process *proc);
	
	EXTERN void sndsig(struct process *proc, int sig);
	
	EXTERN void yield();
	
	EXTERN void sched(struct process *proc);

	EXTERN int issig();
	
	#define KERNEL_RUNNING(p) ((p)->intlvl > 1)
	
	/*
	 * Asserts if the process is the session leader.
	 */
	#define IS_LEADER(p) \
		(p->pgrp->pid == p->pid)

	#define IS_SUPERUSER(p) \
		((p->uid == SUPERUSER) || (p->euid == SUPERUSER))
		
	/* Process table. */
	EXTERN struct process proctab[PROC_MAX];
	
	/*  Current running process. */
	EXTERN struct process *curr_proc;
	
	/* Next available PID. */
	EXTERN pid_t next_pid;
	
	/*  init process. */
	#define INIT (&proctab[1])
	
	/* idle process. */
	#define IDLE (&proctab[0])
	
	/* First process. */
	#define FIRST_PROC ((&proctab[1]))
	
	/* Last process. */
	#define LAST_PROC ((&proctab[PROC_MAX - 1]))
	
	/* Current number of process in the system. */
	EXTERN int nprocs;

#endif /* _ASM_FILE */

#endif /* PM_H_ */
