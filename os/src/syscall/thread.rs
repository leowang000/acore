use crate::{
    mm::kernel_satp,
    task::{add_task, current_task, TaskControlBlock},
    trap::{trap_handler, TrapContext},
};
use alloc::sync::Arc;

pub fn sys_thread_create(entry: usize, arg: usize) -> isize {
    let task = current_task();
    let process = task.process.upgrade().unwrap();
    // Create new thread.
    let new_task = Arc::new(TaskControlBlock::new(
        process.clone(),
        task.inner_exclusive_access()
            .user_resource
            .as_ref()
            .unwrap()
            .user_stack_base,
        true,
    ));
    let new_task_inner = new_task.inner_exclusive_access();
    let user_stack_top = new_task_inner
        .user_resource
        .as_ref()
        .unwrap()
        .user_stack_top();
    let kernel_stack_top = new_task.kernel_stack.get_top();
    let trap_cx = new_task_inner.get_trap_cx();
    drop(new_task_inner);
    *trap_cx = TrapContext::app_initial_context(
        entry,
        user_stack_top,
        kernel_satp(),
        kernel_stack_top,
        trap_handler as usize,
    );
    trap_cx.gprs[10] = arg;
    // Add the new thread to process.
    let tasks = &mut process.inner_exclusive_access().tasks;
    let new_task_tid = new_task.get_tid();
    while tasks.len() < new_task_tid + 1 {
        tasks.push(None);
    }
    tasks[new_task_tid] = Some(new_task.clone());
    // Add the new thread to the task manager.
    add_task(new_task);
    // Return the tid of the new thread.
    new_task_tid as isize
}

pub fn sys_gettid() -> isize {
    current_task().get_tid() as isize
}

pub fn sys_waittid(tid: usize) -> isize {
    // println!("[debug] [kernel] sys_waittid {}", tid);
    let task = current_task();
    if task.get_tid() == tid {
        // A thread cannot wait for itself.
        return -1;
    }
    let process = task.process.upgrade().unwrap();
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.tasks.len() < tid {
        // Waited task does not exist.
        return -1;
    }
    let mut exit_code: Option<i32> = None;
    if let Some(waited_task) = process_inner.tasks[tid].as_ref() {
        if let Some(waited_exit_code) = waited_task.inner_exclusive_access().exit_code {
            exit_code = Some(waited_exit_code);
        }
    } else {
        // Waited task does not exist.
        return -1;
    }
    if let Some(exit_code) = exit_code {
        // Release the kernel stack.
        process_inner.tasks[tid] = None;
        process_inner.dealloc_tid(tid);
        exit_code as isize
    } else {
        // Waited thread has not exited.
        -2
    }
}
