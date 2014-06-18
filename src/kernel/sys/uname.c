/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/uname.c - uname() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <sys/utsname.h>

/*
 * Internal uname().
 */
PRIVATE void do_uname(struct utsname *name)
{
	/* Fill up utsname structure. */
	kstrncpy(name->sysname, SYSNAME, _UTSNAME_LENGTH);
	kstrncpy(name->nodename, NODENAME, _UTSNAME_LENGTH);
	kstrncpy(name->release, RELEASE, _UTSNAME_LENGTH);
	kstrncpy(name->version, VERSION, _UTSNAME_LENGTH);
	kstrncpy(name->machine, MACHINE, _UTSNAME_LENGTH);
}

/*
 * Gets the name of the current system.
 */
PUBLIC int sys_uname(struct utsname *name)
{
	/*
	 * Reset errno, since we may
	 * use it in kernel routines.
	 */
	curr_proc->errno = 0;
	
	/* Invalid buffer. */
	if (chkmem(name, sizeof(struct utsname), MAY_WRITE))
	{
		do_uname(name);
	}
	
	return (curr_proc->errno);
}
