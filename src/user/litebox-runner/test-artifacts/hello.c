// gcc tests/test.c -o test -static -nostdlib (-m32)
 #if defined(__x86_64__)
 int write(int fd, const char *buf, int length)
 {
     int ret;
 
     asm("mov %1, %%eax\n\t"
         "mov %2, %%edi\n\t"
         "mov %3, %%rsi\n\t"
         "mov %4, %%edx\n\t"
         "syscall\n\t"
         "mov %%eax, %0"
         : "=r" (ret)
         : "i" (1), // #define SYS_write 1
           "r" (fd),
           "r" (buf),
           "r" (length)
         : "%eax", "%edi", "%rsi", "%edx");
 
     return ret;
 }
 
 _Noreturn void exit_group(int code)
 {
     /* Infinite for-loop since this function can't return */
     for (;;) {
         asm("mov %0, %%eax\n\t"
             "mov %1, %%edi\n\t"
             "syscall\n\t"
             :
             : "i" (231), // #define SYS_exit_group 231
               "r" (code)
             : "%eax", "%edi");
     }
 }
 #elif defined(__i386__)
 int write(int fd, const char *buf, int length)
 {
     int ret;
 
     asm("mov %1, %%eax\n\t"
         "mov %2, %%ebx\n\t"
         "mov %3, %%ecx\n\t"
         "mov %4, %%edx\n\t"
         "int $0x80\n\t"
         "mov %%eax, %0"
         : "=r" (ret)
         : "i" (4), // #define SYS_write 4
           "g" (fd),
           "g" (buf),
           "g" (length)
         : "%eax", "%ebx", "%ecx", "%edx");
 
     return ret;
 }
 _Noreturn void exit_group(int code)
 {
     /* Infinite for-loop since this function can't return */
     for (;;) {
         asm("mov %0, %%eax\n\t"
             "mov %1, %%ebx\n\t"
             "int $0x80\n\t"
             :
             : "i" (252), // #define SYS_exit_group 252
               "r" (code)
             : "%eax", "%ebx");
     }
 }
 #else
 #error "Unsupported architecture"
 #endif
 
 int main() {
     // use write to print a string
     write(1, "Hello, World!\n", 14);
     return 0;
 }
 
 void _start() {
     exit_group(main());
 }
