#include <fcntl.h>
#include <stdarg.h>
#include <sys/sem.h>
#include <nanvix/klib.h>
#include <semaphore.h>
#include <errno.h>

/**
 *	@brief Add the semaphore to the sem table
 *
 *	If it does not exist, the semaphore is added to the table
 * 	
 *	@returns the index of the named semaphore in the semaphore table
 *			 in case of successful completion, SEM_FAILED otherwise
 *			
 */
int add_entry(int value, const char* name, int mode)
{
	int idx;

	for (idx=0; idx<SEM_OPEN_MAX; idx++)
	{
		if(semtable[idx].nbproc==0 && semtable[idx].name[0]=='\0')
		{
			semtable[idx].value=value;
			kstrcpy(semtable[idx].name,name);
			semtable[idx].mode=mode;
			semtable[idx].nbproc=0;
			/* Semaphore uid is set to the euid of the calling process */
			semtable[idx].uid=curr_proc->euid;
			/* Semaphore gid is set to the egid of the calling process */
			semtable[idx].gid=curr_proc->egid;
			return idx;
		}
	}

	/* Semaphore table full */
	curr_proc->errno = ENFILE;
	return SEM_FAILED; 
}

/**
 * @brief opens a semaphore
 *		 
 * @param	name 	Name of the semaphore
 *			oflag	Creation flags
 *			mode	User permissions
 *			value 	Semaphore value
 *
 * @returns the index of the semaphore in the
 *          semaphore table if it exists, -1
 *          otherwise
 */
/* TODO for error detection :
 *			ENOSPC : There is insufficient space on a storage device for the creation of the new named semaphore.
 *			EMFILE : Too many semaphore descriptors or file descriptors are currently in use by this process.
 */
PUBLIC int sys_semopen(const char* name, int oflag, ...)
{
	mode_t mode;
	int value;
	va_list arg;	/* Variable argument */
	int idx;		/* Index of the opened semaphore */

	idx = existance(name);

	if(oflag & O_CREAT)
	{
		if(idx!=(-1))	/* This semaphore already exists */
		{
			if (oflag & O_EXCL)
			{
				curr_proc->errno = EEXIST;
				return SEM_FAILED;
			}

			if ( namevalid(name) ==(-1) )
			{
				/* Name invalid */
				curr_proc->errno = EINVAL;
				return SEM_FAILED;
			}

			/* Checking if there is at least WRITE or READ permissions */
			if (!permission(semtable[idx].mode, semtable[idx].uid, semtable[idx].gid, curr_proc, MAY_WRITE|MAY_READ, 0))
			{
				curr_proc->errno = EACCES;
				return SEM_FAILED;
			}

		}
		else
		{
			/* Semaphore creation if it does not exist */
			va_start(arg, oflag);
			mode = va_arg(arg, mode_t);
			value = va_arg(arg, int);
			va_end(arg);

			if ( !SEM_VALID_VALUE(value) )
			{
				/* Value greater than maximum */
				curr_proc->errno = EINVAL;
				return SEM_FAILED;
			}

			idx=add_entry (value,name,mode);
		}
	}
	else
	{
		if(idx==(-1))
		{
			/* O_CREAT not set and sem does not exist */
			curr_proc->errno = ENOENT;
			return SEM_FAILED;
		}
	}

	semtable[idx].nbproc++;

	return idx;
}