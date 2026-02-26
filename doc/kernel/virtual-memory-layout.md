# Virtual Memory Layout

Processes in Nanvix have the following virtual memory layout.

| Address Range (start, end)                                    | Owner  | Description                                           |
|---------------------------------------------------------------|--------|-------------------------------------------------------|
| [`KERNEL_BASE_RAW`, `KPOOL_BASE_RAW`)                         | Kernel | Kernel binary                                         |
| [`KPOOL_BASE_RAW`, `USER_BASE_RAW`)                           | Kernel | Kernel page pool                                      |
| [`USER_BASE_RAW`, `USER_MMAP_BASE_RAW`)                       | User   | User binary                                           |
| [`USER_MMAP_BASE_RAW`, `USER_MMAP_END_RAW`)                   | User   | User heap, shared libraries, and memory-mapped object |
| [`USER_MMAP_END_RAW`, `USER_STACK_TOP_RAW`)                   | User   | Guard region                                          |
| [`USER_STACK_TOP_RAW`, `USER_STACK_BASE_RAW`)                 | User   | User stack                                            |
| [`USER_STACK_BASE_RAW`, `USER_END_RAW`)                       | User   | Unused                                                |
