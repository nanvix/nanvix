#include <sys/sem.h>
#include <nanvix/klib.h>

/* 
 *	@TODO : returns -1 if 
 * 	sem isn't a valid semaphore
 *
 */



PUBLIC int sys_semclose(int idx)
{
	/* calling process finished using the semaphore */

	/*	veryfing valid semaphore */
	if(idx<0 || idx>MAX_SEMAPHORES)
	{
		return -1; /* not a valid semaphore */
	}
	else
	{

		semtable[idx].nbproc--; /* 1 less proc using it */
		kprintf("closing : %d proc using the sem called %s\n",semtable[idx].nbproc, semtable[idx].name);

		/* 	if no more process is
		 * 	using the semaphore
		 * 	then deletes it
		 */
		/* when all processes that have opened the semaphore close it, the semaphore is no longer accessible.
		 * The semaphore is no longer accessible when 0 process use it
		 * only if it has been unlinked once 
		 */
		/* reminder : change unlinked */
		if(semtable[idx].nbproc==0 && semtable[idx].unlinked==0)
		{
			kprintf("No one use the sem anymore : removing it\n");
			semtable[idx].value=0;
			(semtable[idx].name)[0]='\0';
			semtable[idx].mode=0;
			semtable[idx].nbproc=0;		/* freeing the slot */
			semtable[idx].unlinked=0;
		}

	}
	return 0; /* sucessful completion */

}