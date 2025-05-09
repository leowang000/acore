//! Constants for qumu

// the frequency of the timer
pub const CLOCK_FREQ: usize = 12500000;
pub const MEMORY_END: usize = 0x80800000;

// (VIRT_TEST, RTC)  in virt machine
#[allow(unused)]
pub const MMIO: &[(usize, usize)] = &[(0x00100000, 0x002000)];
