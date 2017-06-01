#include <string.h>
#include <fcntl.h>
#include <stdarg.h>	/*va_start ... */
#include <sys/sem.h>
#include <nanvix/klib.h>
#include <semaphore.h>
#include <errno.h>
//maybe unecessary #include <sys/mm.h>

	/**
	 *	@brief Add the semaphore to the sem table
	 *
	 *	If it does not exist, the semaphore is added to the table
	 * 	
	 *	@returns the index of the named semaphore in the semaphore table
	 */
	int add_entry(int value, const char* name, int mode)
	{
		int idx;

		kprintf("nom du semaphore que l'on va ajouter : %s",name);

		for (idx=0; idx<SEM_OPEN_MAX; idx++)
		{
			kprintf("checking for empty slot\n");
			if(semtable[idx].nbproc==0 && semtable[idx].name[0]=='\0')
			{
				kprintf("nb proc = 0 : place vide trouvée \n");
				semtable[idx].value=value;
				kstrcpy(semtable[idx].name,name);
				semtable[idx].mode=mode;
				semtable[idx].nbproc=0;
				/* semaphore uid is set to the euid of the calling process */
				semtable[idx].uid=curr_proc->euid;
				/* semaphore gid is set to the egid of the calling process */
				semtable[idx].gid=curr_proc->egid;
				return idx;
			}
		}

		/* semaphore table full */
		curr_proc->errno = ENFILE;
		return SEM_FAILED; 
	}

	/*
	 * @brief checks the existance of a semaphore
	 *		  in the semaphore table
	 *		  
	 * @returns the index of the semaphore in the
	 *          semaphore table if it exists, -1
	 *          otherwise
	 */
	int existance(const char* semname)
	{
		int idx;

		for (idx=0; idx<SEM_OPEN_MAX; idx++)
		{
			kprintf("comparaison : %s et %s",semtable[idx].name,semname);
			
			if(!(kstrcmp(semtable[idx].name,semname))){
				/* add process to semaphore  */
				return idx;
			}
		}

		return -1;
	}


	/* TODO : 
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
 			/* if O_CREAT and O_EXCL flags are set
 			 * and the semaphore name already exists
 			 * return an error
 			 */
 			if(idx!=(-1))	/* if this name already exists */
 			{
 			 	kprintf("name exists\n");

 				if (oflag & O_EXCL)
 				{
 					/* O_EXCL flag result in an error if sem exists */
 					curr_proc->errno = EEXIST;
 					return SEM_FAILED;
 				}

 				if ( namevalid(name) ==(-1) )
 				{
 					kprintf("check");
 					/* sem name is not valid */
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
 				kprintf("name doesn't exist\n");

	 			/* Semaphore creation if doesn't 'exist */
	 			va_start(arg, oflag);
	 			mode = va_arg(arg, mode_t);
	 			value = va_arg(arg, int);
	 			va_end(arg);

 				if ( !SEM_VALID_VALUE(value) )
 				{
 					/* value greater than maximum */
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

		kprintf("opening : %d proc using the sem called %s\n",semtable[idx].nbproc, semtable[idx].name);
 		/* idx est l'index du semaphore ouvert */

		return idx;
	}