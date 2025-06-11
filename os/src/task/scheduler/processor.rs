//! the state of the processor
use crate::{
    sbi::shutdown,
    sync::UPSafeCell,
    task::{
        process::ProcessControlBlock,
        scheduler::{switch::__switch, task_manager::fetch_task},
        thread::TaskStatus,
        TaskContext, TaskControlBlock,
    },
    trap::TrapContext,
};
use alloc::sync::Arc;
use lazy_static::lazy_static;

pub struct Processor {
    current: Option<Arc<TaskControlBlock>>,
    idle_task_cx: TaskContext,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }

    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }

    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.clone()
    }

    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = UPSafeCell::new(Processor::new());
}

pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

pub fn current_task() -> Arc<TaskControlBlock> {
    PROCESSOR.exclusive_access().current().unwrap()
}

pub fn current_process() -> Arc<ProcessControlBlock> {
    current_task().process.upgrade().unwrap()
}

pub fn current_task_satp() -> usize {
    current_task().satp()
}

pub fn current_task_trap_cx() -> &'static mut TrapContext {
    current_task().inner_exclusive_access().get_trap_cx()
}

pub fn current_task_trap_cx_user_va() -> usize {
    current_task()
        .inner_exclusive_access()
        .user_resource
        .as_ref()
        .unwrap()
        .trap_cx_bottom_va()
}

pub fn current_kernel_stack_top() -> usize {
    current_task().kernel_stack.get_top()
}

pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.exclusive_access();
        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            let mut task_inner = task.inner_exclusive_access();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.status = TaskStatus::Running;
            drop(task_inner);
            processor.current = Some(task);
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            shutdown(false);
        }
    }
}

pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}
