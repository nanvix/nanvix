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
			semtable[idx].state=mode;
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
 *			EACESS : Check creation permission if semaphore does not exist
 */
PUBLIC int sys_semopen(const char* name, int oflag, ...)
{
	mode_t mode;
	int value;
	va_list arg;	/* Variable argument */
	int idx;		/* Index of the opened semaphore */

	if ( namevalid(name)==(-1) )
	{
		/* Name invalid */
		curr_proc->errno = EINVAL;
		return SEM_FAILED;
	}

	idx = existance(name);

	if(idx==(-1))	/* This semaphore does not exist */
	{
		if(oflag & O_CREAT)	
		{
			if (oflag & O_EXCL)
			{
				/* Both O_CREAT and O_EXCL flags set */
				curr_proc->errno = EEXIST;
				return SEM_FAILED;
			}
			
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
			kprintf("Creating sem : %s at index %d",semtable[idx].name,idx);

		}
		else
		{
			/* O_CREAT not set and sem does not exist */
			curr_proc->errno = ENOENT;
			return SEM_FAILED;
		}
	}
	else	/* This semaphore already exists */
	{
		/* Checking if there is WRITE and READ permissions */
		if (	!permission(semtable[idx].state, semtable[idx].uid, semtable[idx].gid, curr_proc, MAY_WRITE, 0) \
			 ||	!permission(semtable[idx].state, semtable[idx].uid, semtable[idx].gid, curr_proc, MAY_READ, 0) )
		{
			curr_proc->errno = EACCES;
			return SEM_FAILED;
		}

		kprintf("Opening sem : %s at index %d",semtable[idx].name,idx);

	}

	semtable[idx].nbproc++;

	return idx;
}