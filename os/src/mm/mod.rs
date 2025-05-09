mod address;
mod address_space;
mod frame_allocator;
mod heap_allocator;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
pub use address_space::{AddressSpace, KERNEL_SPACE, Permission};
pub use frame_allocator::{FrameTracker, frame_alloc};
pub use page_table::{PageTableEntry, translated_byte_buffer, translated_refmut, traslated_str};

pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();
}

#[allow(unused)]
pub fn test() {
    heap_allocator::heap_test();
    frame_allocator::frame_allocator_test();
    address_space::remap_test();
}
