#include <fcntl.h>
#include <stdarg.h>
#include <sys/sem.h>
#include <nanvix/klib.h>
#include <semaphore.h>
#include <errno.h>
#include <nanvix/fs.h>

/**
 *	@brief Add the semaphore to the sem table
 *
 *	If it does not exist, the semaphore is added to the table
 * 	
 *	@returns the index of the named semaphore in the semaphore table
 *			 in case of successful completion, SEM_FAILED otherwise
 *			
 */
// PRIVATE ino_t add_entry(int value, const char* name, int mode)
// {
// 	struct inode *semdir, *inode; /* /home/mysemaphores/ */

// 	semdir = inode_name("/home/mysem");

// 	/* 
// 	 *  if ( too much semaphores )
// 	 * 	Semaphore table full 
// 	 * 
// 	 *  return (-ENFILE); 
// 	 */

// 	/* Initialize Semaphore inode. */
// 	inode = do_creat(semdir, name, mode, O_CREAT);

// 	inode->size = sizeof(struct ksem);
// 	inode->time = CURRENT_TIME;
// 	inode->count = 1;

// 	sembuf.value=value;
// 	kstrcpy(sembuf.name,name);
// 	sembuf.state=mode;
// 	sembuf.nbproc=0;
// 	sembuf.uid=curr_proc->euid;
// 	sembuf.gid=curr_proc->egid;
// 	sembuf.currprocs[0]=curr_proc->pid;

// 	file_write(inode, &sembuf, sizeof(struct ksem),0);

// 	return sembuf.num;
// }

/**
 * @brief Opens a semaphore
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
PUBLIC ino_t sys_semopen(const char* name, int oflag, ...)
{
	mode_t mode;
	int value;
	va_list arg;				/* Variable argument */
	int i, freeslot;		/* Index of the opened semaphore */
	struct inode *inode;
	freeslot = -1;

	/* Name invalid */
	if (namevalid(name) == (-1))
		return (-EINVAL);

	inode = existence_inode(name);

	if (inode == (NULL))	/* This semaphore does not exist */
	{
		if(oflag & O_CREAT)	
		{
			/* Both O_CREAT and O_EXCL flags set */
			if (oflag & O_EXCL)
				return (-EEXIST);
			
			/* Semaphore creation if it does not exist */
			va_start(arg, oflag);
			mode = va_arg(arg, mode_t);
			value = va_arg(arg, int);
			va_end(arg);

			/* Value greater than maximum */
			if (!SEM_VALID_VALUE(value))
				return (-EINVAL);

			inode = inode_semaphore (value,name,mode);
		}
		/* O_CREAT not set and sem does not exist */
		else
			return (-ENOENT);
	}
	else	/* This semaphore already exists */
	{
		/* Checking if there is WRITE and READ permissions */
		if (	!permission(sembuf.state, inode->uid, inode->gid, curr_proc, MAY_WRITE, 0) \
			 ||	!permission(sembuf.state, inode->uid, inode->gid, curr_proc, MAY_READ, 0) )
			return (EACCES);
		
		for (i = 0; i < PROC_MAX; i++)
		{

			if (sembuf.currprocs[i] == (-1) && freeslot < 0)
			{

				freeslot = i;
			}
			
			/* Preventing multiple opening */
			if (sembuf.currprocs[i] == curr_proc->pid)
			{
				return (-1);
			}
		}

		sembuf.currprocs[freeslot] = curr_proc->pid;

		sembuf.nbproc++;

		file_write(inode, &sembuf, sizeof(struct ksem),0);
	}

	return inode->num;
}
