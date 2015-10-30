/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <sys/utsname.h> - System name structure.
 */

#ifndef UTSNAME_H_
#define UTSNAME_H_
#ifndef _ASM_FILE_

	/* Length of strings in utsname structure */
	#define _UTSNAME_LENGTH 9

	/*
	 * System name structure.
	 */
	struct utsname
	{
		char sysname[_UTSNAME_LENGTH];  /* Operating system name. */
		char nodename[_UTSNAME_LENGTH]; /* Network node name.     */
		char release[_UTSNAME_LENGTH];  /* Kernel release.        */
		char version[_UTSNAME_LENGTH];  /* Kernel version.        */
		char machine[_UTSNAME_LENGTH];  /* Hardware name.         */
	};
	
	/*
	 * Gets the name of the current system.
	 */
	extern int uname(struct utsname *name);

#endif /* _ASM_FILE_ */
#endif /* UTSNAME_H_ */
