#include <string.h>
#include <fcntl.h>
#include <stdarg.h>	/*va_start ... */
#include <sys/sem.h>
#include <nanvix/klib.h>
#include <semaphore.h>
//maybe unecessary #include <sys/mm.h>

	/**
	 *	@brief Add the semaphore to the sem table
	 *
	 *	If it does not exist, the semaphore is added to the table
	 * 	
	 *	@returns the index of the named semaphore in the semaphore table
	 */
	int add_entry(int value, const char* name, int mode){
		int idx;

		kprintf("nom du semaphore que l'on va ajouter : %s",name);

		for (idx=0; idx<SEM_OPEN_MAX; idx++)
		{
			//kprintf("checking for empty slot\n");
			if(semtable[idx].nbproc==0)
			{
				//kprintf("nb proc = 0 : place vide trouvée \n");
				semtable[idx].value=value;
				kstrcpy(semtable[idx].name,name);
				semtable[idx].mode=mode;
				semtable[idx].nbproc=0;
				return idx;
			}
		}

		return -1; /* semaphore table full */
	}

	/*
	 * @brief checks the existance of a semaphore
	 *		  in the semaphore table
	 *		  
	 * @returns the index of the semaphore in the
	 *          semaphore table if it exists, -1
	 *          otherwise
	 */
	int existance(const char* semname){
		int idx;

		for (idx=0; idx<SEM_OPEN_MAX; idx++)
		{
			kprintf("comparaison : %s et %s",semtable[idx].name,semname);
			//if(s->name == semname){
			if(!(kstrcmp(semtable[idx].name,semname))){
				/* add process to semaphore  */
				return idx;
			}
		}

		return -1;
	}


/* TODO : verify char* adresses so the user doesn't break anything */
	PUBLIC int sys_semopen(const char* name, int oflag, ...){

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

		kprintf("opening : %d proc using the sem called %s\n",semtable[idx].nbproc, semtable[idx].name);
 		/* idx est l'index du semaphore ouvert */

		return idx;
	}