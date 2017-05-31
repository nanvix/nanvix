#include <string.h>
#include <fcntl.h>
#include <stdarg.h>	/*va_start ... */
#include <sys/sem.h>
#include <nanvix/klib.h>
#include <semaphore.h>


	/* add the semaphore to the sem table if it doesn't exist
	 * returns the added/existing semaphore address
	 */
	int add_entry(int value, char* name, int mode){
		int idx;

		kprintf("nom du semaphore que l'on va ajouter : %s",name);

		for (idx=0; idx<MAX_SEMAPHORES; idx++)
		{
			kprintf("checking for empty slot\n");
			if(semtable[idx].nbproc==0)
			{
				kprintf("nb proc = 0 : place vide trouvée \n");
				semtable[idx].value=value;
				kstrcpy(name,semtable[idx].name);
				semtable[idx].mode=mode;
				semtable[idx].nbproc=0;
				return idx;
			}
		}

		return -1; /* semaphore table full */
	}

	/*
	 * returns 0 if doesn't exists
	 * returns non-zero otherwise
	 */
	int existance(char* semname){
		int idx;

		for (idx=0; idx<MAX_SEMAPHORES; idx++)
		{
			//if(s->name == semname){
			if(!(kstrcmp(semtable[idx].name,semname))){
				/* add process to semaphore  */
				return idx;
			}
		}

		return -1;
	}


/* TODO : verify char* adresses so the user doesn't break anything */
	PUBLIC int sys_semopen(char* name, int oflag, ...){

 		mode_t mode;
 		int value;
 		va_list arg;	/* Variable argument */
 		//sem_t *s;		/* Opened semaphore */
 		int idx;		/* Index of the opened semaphore */

 		if(oflag & O_CREAT)
 		{
 			/* if O_CREAT and O_EXCL flags are set
 			 * and the semaphore name already exists
 			 * return an error
 			 */

 			/* correct this to have everything in the if */
 			idx = existance(name);

 			if(idx!=(-1))				/* if this name already exists */
 			{
 			 	kprintf("name exists\n");

 				if(oflag & O_EXCL)
 				{
 					return -1; 			/* error */
 				}
 				else
 				{

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
 				idx=add_entry (value,name,mode);
 			}
 		}

 		semtable[idx].nbproc++;

 		/* idx est l'index du semaphore ouvert */

		return idx;
	}

