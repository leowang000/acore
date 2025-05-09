use crate::loader::get_app_data_by_name;
use alloc::sync::Arc;
use lazy_static::lazy_static;
use processor::take_current_task;
use task::*;

mod context;
mod kernel_stack;
mod manager;
mod pid;
mod processor;
mod switch;
mod task;

pub use context::TaskContext;
pub use kernel_stack::KernelStack;
pub use manager::add_task;
pub use pid::{PidAllocator, PidHandle, pid_alloc};
pub use processor::{
    Processor, current_task, current_task_satp, current_task_trap_cx, run_tasks, schedule,
};

lazy_static! {
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(
        get_app_data_by_name("initproc").unwrap()
    ));
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}

pub fn suspend_current_and_run_next() {
    let current_task = take_current_task().unwrap();
    let mut current_task_inner = current_task.inner_exclusive_access();
    let current_task_cx_ptr = &mut current_task_inner.task_cx as *mut _;
    current_task_inner.status = TaskStatus::Ready;
    drop(current_task_inner);
    add_task(current_task);
    schedule(current_task_cx_ptr);
}

pub fn exit_current_and_run_next(exit_code: i32) {
    let current_task = take_current_task().unwrap();
    let mut current_task_inner = current_task.inner_exclusive_access();
    current_task_inner.status = TaskStatus::Zombie;
    current_task_inner.exit_code = exit_code;
    for child in current_task_inner.children.iter() {
        child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
        INITPROC
            .inner_exclusive_access()
            .children
            .push(child.clone());
    }
    current_task_inner.children.clear();
    current_task_inner.address_space.recycle_data_pages();
    drop(current_task_inner);
    drop(current_task);
    let mut unused_task_cx = TaskContext::zero_init();
    schedule(&mut unused_task_cx as *mut _);
}
