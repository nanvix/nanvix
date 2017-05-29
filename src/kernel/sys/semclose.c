#include <fcntl.h>
#include <sys/sem.h>

/* 
 *	@TODO : returns -1 if 
 * 	sem isn't a valid semaphore
 *
 */



PUBLIC int sys_semclose(sem_t* sem)
{

	sem->nbproc--;

	/* 	if no more process is
	 * 	using the semaphore
	 * 	then delete it
	 */
	if(sem->nbproc==0)
	{
		sem=NULL;
	}
	else
	{

	}


	return 0;


}