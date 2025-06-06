mod address;
mod address_space;
mod frame_allocator;
mod heap_allocator;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
pub use address_space::{kernel_satp, AddressSpace, Permission, KERNEL_SPACE};
pub use frame_allocator::{frame_alloc, frame_dealloc, FrameTracker};
pub use page_table::{
    translated_byte_buffer, translated_refmut, translated_str, PageTable, PageTableEntry,
    PageTableView, UserBuffer, UserBufferIterator,
};

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
