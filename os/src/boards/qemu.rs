pub const CLOCK_FREQ: usize = 12500000;
pub const MEMORY_END: usize = 0x88000000;

pub const VIRT_TEST: usize = 0x100000;
pub const VIRT_TEST_SIZE: usize = 0x1000;
pub const VIRT_RTC: usize = 0x101000;
pub const VIRT_RTC_SIZE: usize = 0x1000;
pub const VIRT_CLINT: usize = 0x2000000;
pub const VIRT_CLINT_SIZE: usize = 0x10000;
pub const VIRT_UART0: usize = 0x10000000;
pub const VIRT_UART0_SIZE: usize = 0x100;
pub const VIRT_VIRTIO: usize = 0x10001000;
pub const VIRT_VIRTIO_SIZE: usize = 0x1000;

pub const MMIO: &[(usize, usize)] = &[
    (VIRT_TEST, VIRT_TEST_SIZE),
    (VIRT_RTC, VIRT_RTC_SIZE),
    (VIRT_CLINT, VIRT_CLINT_SIZE),
    (VIRT_UART0, VIRT_UART0_SIZE),
    (VIRT_VIRTIO, VIRT_VIRTIO_SIZE),
];

pub const MTIME: usize = VIRT_CLINT + 0xbff8;
pub const MTIMECMP: usize = VIRT_CLINT + 0x4000;

pub type BlockDeviceImpl = crate::drivers::block::VirtIOBlock;
