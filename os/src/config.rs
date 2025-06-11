pub use crate::board::*;

pub const USER_STACK_SIZE: usize = 0x2000; // 8KB
pub const KERNEL_STACK_SIZE: usize = 0x2000; // 8KB
pub const KERNEL_HEAP_SIZE: usize = 0x200000; // 2MB

pub const PAGE_SIZE: usize = 0x1000; // 4KB
pub const PAGE_SIZE_BITS: usize = 12;

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT_BASE: usize = TRAMPOLINE - PAGE_SIZE;