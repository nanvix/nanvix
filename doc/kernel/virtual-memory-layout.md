# Virtual Memory Layout

Processes in Nanvix have the following virtual memory layout.

| Address Range (start, end)                                    | Owner  | Description                 |
|---------------------------------------------------------------|--------|-----------------------------|
| [`KERNEL_BASE_RAW`, `KPOOL_BASE_RAW`)                         | Kernel | Kernel binary               |
| [`KPOOL_BASE_RAW`, `USER_BASE_RAW`)                           | Kernel | Kernel page pool            |
| [`USER_BASE_RAW`, `USER_MMAP_BASE_RAW`)                       | User   | User binary                 |
| [`USER_MMAP_BASE_RAW`, `USER_LIBS_BASE_RAW`)                  | User   | Memory mapped objects       |
| [`USER_LIBS_BASE_RAW`, `USER_HEAP_BASE_RAW`)                  | User   | Shared libraries            |
| [`USER_HEAP_BASE_RAW`, `USER_HEAP_END_RAW`)                   | User   | User heap                   |
| [`USER_HEAP_END_RAW`, `USER_STACK_TOP_RAW`)                   | User   | User heap/stack guard       |
| [`USER_STACK_TOP_RAW`, `USER_STACK_BASE_RAW`)                 | User   | User stack                  |
| [`USER_STACK_BASE_RAW`, `USER_END_RAW`)                       | User   | Unused                      |
