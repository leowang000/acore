//! Constants for qumu

/// the frequency of the timer
pub const CLOCK_FREQ: usize = 12500000;
pub const MEMORY_END: usize = 0x88000000;

/// (VIRT_TEST, RTC) and Virtio Block in virt machine
pub const MMIO: &[(usize, usize)] = &[(0x00100000, 0x002000), (0x10001000, 0x001000)];

pub type BlockDeviceImpl = crate::drivers::block::VirtIOBlock;
