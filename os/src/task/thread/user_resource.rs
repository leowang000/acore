use crate::{
    config::{PAGE_SIZE, TRAP_CONTEXT_BASE, USER_STACK_SIZE},
    mm::{Permission, PhysPageNum, VirtAddr},
    task::process::ProcessControlBlock,
};
use alloc::sync::{Arc, Weak};

pub struct TaskUserResource {
    pub tid: usize,
    pub user_stack_base: usize,
    pub process: Weak<ProcessControlBlock>,
}

fn trap_cx_bottom(tid: usize) -> usize {
    TRAP_CONTEXT_BASE - tid * PAGE_SIZE
}

fn user_stack_bottom(user_stack_base: usize, tid: usize) -> usize {
    user_stack_base + tid * (PAGE_SIZE + USER_STACK_SIZE)
}

impl TaskUserResource {
    pub fn new(
        process: Arc<ProcessControlBlock>,
        user_stack_base: usize,
        alloc_user_resource: bool,
    ) -> Self {
        let tid = process.inner_exclusive_access().alloc_tid();
        let task_user_resource = Self {
            tid: tid,
            user_stack_base: user_stack_base,
            process: Arc::downgrade(&process),
        };
        if alloc_user_resource {
            task_user_resource.alloc_user_resource();
        }
        task_user_resource
    }

    /// Allocate the user stack and the trap context in the address space of self.process.
    pub fn alloc_user_resource(&self) {
        let process = self.process.upgrade().unwrap();
        let mut process_inner = process.inner_exclusive_access();
        let user_stack_bottom = user_stack_bottom(self.user_stack_base, self.tid);
        process_inner.address_space.add_segment_framed(
            user_stack_bottom.into(),
            (user_stack_bottom + USER_STACK_SIZE).into(),
            Permission::R | Permission::W | Permission::U,
        );
        let trap_cx_bottom = trap_cx_bottom(self.tid);
        process_inner.address_space.add_segment_framed(
            trap_cx_bottom.into(),
            (trap_cx_bottom + PAGE_SIZE).into(),
            Permission::R | Permission::W,
        );
    }

    /// Deallocate the user stack and the trap context in the address space of self.process.
    fn dealloc_user_resource(&self) {
        let process = self.process.upgrade().unwrap();
        let mut process_inner = process.inner_exclusive_access();
        let user_stack_bottom = user_stack_bottom(self.user_stack_base, self.tid);
        process_inner
            .address_space
            .remove_segment_with_start_vpn(user_stack_bottom.into());
        let trap_cx_bottom = trap_cx_bottom(self.tid);
        process_inner
            .address_space
            .remove_segment_with_start_vpn(trap_cx_bottom.into());
    }

    pub fn trap_cx_bottom_va(&self) -> usize {
        trap_cx_bottom(self.tid)
    }

    pub fn trap_cx_ppn(&self) -> PhysPageNum {
        let process = self.process.upgrade().unwrap();
        let process_inner = process.inner_exclusive_access();
        let trap_cx_bottom_va: VirtAddr = trap_cx_bottom(self.tid).into();
        process_inner
            .address_space
            .translate(trap_cx_bottom_va.into())
            .unwrap()
            .ppn()
    }

    pub fn user_stack_top(&self) -> usize {
        user_stack_bottom(self.user_stack_base, self.tid) + USER_STACK_SIZE
    }
}

/// Only user_stack and the trap_cx are released. tid will be released when the process exits, or the thread is waited.
impl Drop for TaskUserResource {
    fn drop(&mut self) {
        self.dealloc_user_resource();
    }
}
