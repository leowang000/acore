# ACore-2025

## Project Overview

This implementation serves as the final project (term assignment) for the CS2952 course. It is a 64-bit operating system developed using the Rust programming language, targeting the RISC-V architecture and designed specifically for single-core processor environments. The system runs within the QEMU emulator, providing a controlled and accessible development environment. This implementation builds upon the educational foundation of rCore, a teaching-oriented operating system developed at Tsinghua University.

## Implemented Components

- Bootloader
    - [x] Initialization
    - [x] Entering S mode for the kernel
- Allocator
    - [x] Buddy allocator
    - [x] Frame allocator (or any fine-grained allocator for any size of memory)
    - SLAB (Optional)
- Page table
    - [x] For kernel
    - [x] For each user process
- Console
    - [x] Read
    - [x] Write
- Message & data transfer
    - [x] User -> Kernel
    - [x] Kernel -> User
    - [x] Kernel -> Kernel
    - [x] User -> User
- Process
    - Process loading
        - [x] ELF parsing
        - [x] Sections loading (ref to page table)
    - Syscall
        - [x] Kick off a new process (Something like fork and exec)
        - [x] Wait for child processes (Something like wait)
        - [x] Exit from a process (Something like exit)
    - Process manager
        - [x] Process creation
        - [x] Process termination
    - Scheduler
        - [x] Context switch
        - [x] Scheduling mechanism (must be time sharing)
            - [ ] Advanced scheduling mechanism (Optional)
        - [x] Timer interrupt
        - [ ] IPI (Optional)
    - IPC
        - [x] Pipe
    - Synchronization primitives
        - [x] Mutex
        - [x] Conditional variables (Optional)
        - [x] Semaphores
    - File system (Optional)
        - [x] File/directory creation/deletion
        	- partially implemented
        - [ ] File/directory renaming
        - [x] File read
        - [x] File write
        - [ ] File/directory moving
        - [ ] (optional) access control, atime/mtime/…
- Multicore (Optional)
- Driver (Optional)