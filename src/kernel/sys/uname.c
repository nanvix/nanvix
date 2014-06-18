/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/uname.c - uname() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <sys/utsname.h>
#include <errno.h>

/*
 * Gets the name of the current system.
 */
PUBLIC int sys_uname(struct utsname *name)
{
	/* Invalid buffer. */
	if (!chkmem(name, sizeof(struct utsname), MAY_WRITE))
		return (-EINVAL);
	
	/* Fill up utsname structure. */
	kstrncpy(name->sysname, SYSNAME, _UTSNAME_LENGTH);
	kstrncpy(name->nodename, NODENAME, _UTSNAME_LENGTH);
	kstrncpy(name->release, RELEASE, _UTSNAME_LENGTH);
	kstrncpy(name->version, VERSION, _UTSNAME_LENGTH);
	kstrncpy(name->machine, MACHINE, _UTSNAME_LENGTH);
	
	return (0);
}
