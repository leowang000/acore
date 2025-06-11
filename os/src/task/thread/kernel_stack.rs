use crate::{
    config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE},
    mm::{Permission, VirtAddr, KERNEL_SPACE},
    sync::UPSafeCell,
    task::RecycleAllocator,
};
use lazy_static::lazy_static;

lazy_static! {
    static ref KERNEL_STACK_ALLOCATOR: UPSafeCell<RecycleAllocator> =
        UPSafeCell::new(RecycleAllocator::new());
}

// return (bottom, top) of a kernel stack in kernel address space
pub fn kernel_stack_position(id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    (top - KERNEL_STACK_SIZE, top)
}

pub struct KernelStack {
    pub id: usize,
}

impl KernelStack {
    pub fn get_top(&self) -> usize {
        kernel_stack_position(self.id).1
    }

    pub fn get_bottom(&self) -> usize {
        kernel_stack_position(self.id).0
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let kernel_stack_bottom_va: VirtAddr = self.get_bottom().into();
        KERNEL_SPACE
            .exclusive_access()
            .remove_segment_with_start_vpn(kernel_stack_bottom_va.into());
        KERNEL_STACK_ALLOCATOR.exclusive_access().dealloc(self.id);
    }
}

pub fn alloc_kernel_stack() -> KernelStack {
    let id = KERNEL_STACK_ALLOCATOR.exclusive_access().alloc();
    let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(id);
    KERNEL_SPACE.exclusive_access().add_segment_framed(
        kernel_stack_bottom.into(),
        kernel_stack_top.into(),
        Permission::R | Permission::W,
    );
    KernelStack { id: id }
}
