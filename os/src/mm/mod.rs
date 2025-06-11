use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use lazy_static::lazy_static;

mod address;
mod address_space;
mod frame_allocator;
mod heap_allocator;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
pub use address_space::{AddressSpace, Permission};
pub use frame_allocator::{frame_alloc, frame_dealloc, FrameTracker};
pub use page_table::{
    translated_byte_buffer, translated_ref, translated_refmut, translated_str, PageTable,
    PageTableEntry, PageTableView, UserBuffer, UserBufferIterator,
};

lazy_static! {
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<AddressSpace>> =
        Arc::new(UPSafeCell::new(AddressSpace::new_kernel()));
}

pub fn kernel_satp() -> usize {
    KERNEL_SPACE.exclusive_access().satp()
}

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
