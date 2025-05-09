use crate::{
    config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE},
    mm::{KERNEL_SPACE, Permission, VirtAddr},
};

use super::PidHandle;

// return (bottom, top) of a kernel stack in kernel address space
pub fn kernel_stack_position(pid: usize) -> (usize, usize) {
    let top = TRAMPOLINE - pid * (KERNEL_STACK_SIZE + PAGE_SIZE);
    (top - KERNEL_STACK_SIZE, top)
}

pub struct KernelStack {
    pid: usize,
}

impl KernelStack {
    pub fn new(pid: &PidHandle) -> Self {
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid.0);
        KERNEL_SPACE.exclusive_access().add_segment_framed(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            Permission::R | Permission::W,
        );
        Self { pid: pid.0 }
    }

    pub fn push_on_top<T>(&self, value: T) -> *mut T
    where
        T: Sized,
    {
        let kernel_stack_top = self.get_top();
        let ptr_mut = (kernel_stack_top - core::mem::size_of::<T>()) as *mut T;
        unsafe {
            *ptr_mut = value;
        }
        ptr_mut
    }

    pub fn get_top(&self) -> usize {
        kernel_stack_position(self.pid).1
    }

    pub fn get_bottom(&self) -> usize {
        kernel_stack_position(self.pid).0
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let kernel_stack_bottom_va: VirtAddr = self.get_bottom().into();
        KERNEL_SPACE
            .exclusive_access()
            .remove_segment_with_start_vpn(kernel_stack_bottom_va.into());
    }
}
