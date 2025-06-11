use crate::{
    mm::PhysPageNum, sync::UPSafeCell, task::process::ProcessControlBlock, trap::TrapContext,
};
use alloc::sync::{Arc, Weak};
use core::cell::RefMut;

mod context;
mod kernel_stack;
mod user_resource;

pub use context::TaskContext;
pub use kernel_stack::{alloc_kernel_stack, KernelStack};
pub use user_resource::TaskUserResource;

#[derive(Clone, Copy, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Blocked,
}
pub struct TaskControlBlockInner {
    pub user_resource: Option<TaskUserResource>,
    pub status: TaskStatus,
    pub task_cx: TaskContext,
    pub trap_cx_ppn: PhysPageNum,
    pub exit_code: Option<i32>,
}

pub struct TaskControlBlock {
    pub process: Weak<ProcessControlBlock>,
    pub kernel_stack: KernelStack,
    inner: UPSafeCell<TaskControlBlockInner>,
}

impl TaskControlBlockInner {
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }
}

impl TaskControlBlock {
    pub fn new(
        process: Arc<ProcessControlBlock>,
        user_stack_base: usize,
        alloc_user_resource: bool,
    ) -> Self {
        let user_resource =
            TaskUserResource::new(process.clone(), user_stack_base, alloc_user_resource);
        let trap_cx_ppn = user_resource.trap_cx_ppn();
        let kernel_stack = alloc_kernel_stack();
        let kernel_stack_top = kernel_stack.get_top();
        Self {
            process: Arc::downgrade(&process),
            kernel_stack: kernel_stack,
            inner: UPSafeCell::new(TaskControlBlockInner {
                user_resource: Some(user_resource),
                status: TaskStatus::Ready,
                task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                trap_cx_ppn: trap_cx_ppn,
                exit_code: None,
            }),
        }
    }

    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }

    pub fn satp(&self) -> usize {
        self.process
            .upgrade()
            .unwrap()
            .inner_exclusive_access()
            .address_space
            .satp()
    }

    pub fn get_tid(&self) -> usize {
        self.inner_exclusive_access()
            .user_resource
            .as_ref()
            .unwrap()
            .tid
    }
}
