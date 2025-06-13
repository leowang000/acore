use crate::{
    fs::{open_file, OpenFlags},
    task::{
        process::ProcessControlBlock,
        thread::{TaskStatus, TaskUserResource},
    },
    timer::remove_timer,
};
use alloc::{sync::Arc, vec::Vec};
use lazy_static::lazy_static;

mod process;
mod scheduler;
mod signal;
mod thread;
mod utils;

pub use process::{pid_alloc, PidHandle};
pub use scheduler::{
    add_task, current_kernel_stack_top, current_process, current_task, current_task_satp,
    current_task_trap_cx, current_task_trap_cx_user_va, pid2process, remove_from_pid2process,
    remove_task, schedule, take_current_task, wakeup_task,
};
pub use signal::{SignalAction, SignalActionTable, SignalFlags};
pub use thread::{KernelStack, TaskContext, TaskControlBlock};
pub use utils::RecycleAllocator;

lazy_static! {
    pub static ref INITPROC: Arc<ProcessControlBlock> = {
        let inode = open_file("initproc", OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        ProcessControlBlock::new(v.as_slice())
    };
}

/// Initialize INITPROC. INITPROC will not be initialized before it is accessed.
fn add_initproc() {
    let _ = INITPROC.clone();
}

pub fn run_tasks() {
    add_initproc();
    scheduler::run_tasks();
}

pub fn suspend_current_and_run_next() {
    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let current_task_cx_ptr = &mut task_inner.task_cx as *mut _;
    task_inner.status = TaskStatus::Ready;
    drop(task_inner);
    add_task(task);
    schedule(current_task_cx_ptr);
}

pub fn block_current_and_run_next() {
    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let current_task_cx_ptr = &mut task_inner.task_cx as *mut _;
    task_inner.status = TaskStatus::Blocked;
    drop(task_inner);
    drop(task);
    schedule(current_task_cx_ptr);
}

pub fn exit_current_and_run_next(exit_code: i32) {
    let task = take_current_task().unwrap();
    let process = task.process.upgrade().unwrap();
    let tid = task.get_tid();
    let mut task_inner = task.inner_exclusive_access();
    // Record the exit code of the thread.
    task_inner.exit_code = Some(exit_code);
    // Release thread user resources.
    task_inner.user_resource = None;
    // task_inner and task must be dropped manually, because schedule never returns.
    drop(task_inner);
    drop(task);
    // If the main thread exits, the process should be terminated.
    if tid == 0 {
        let pid = process.get_pid();
        remove_from_pid2process(pid);
        let mut process_inner = process.inner_exclusive_access();
        // Mark this process as a zombie process.
        process_inner.is_zombie = true;
        // The exit code of a process is the exit code of its main thread.
        process_inner.exit_code = exit_code;
        for child in process_inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            INITPROC
                .inner_exclusive_access()
                .children
                .push(child.clone());
        }
        let mut recycle_resources: Vec<TaskUserResource> = Vec::new();
        for task in process_inner.tasks.iter().filter(|t| t.is_some()) {
            let task = task.as_ref().unwrap();
            remove_inactive_task(task.clone());
            let mut task_inner = task.inner_exclusive_access();
            if let Some(resource) = task_inner.user_resource.take() {
                recycle_resources.push(resource);
            }
        }
        drop(process_inner);
        // Deallocate the user resources first. Otherwise, these pages will be deallocated twice.
        recycle_resources.clear();
        let mut process_inner = process.inner_exclusive_access();
        // Clear children vector.
        process_inner.children.clear();
        // Deallocate the program code/data sections in user address space.
        process_inner.address_space.recycle_data_pages();
        // Drop file descriptors.
        process_inner.fd_table.clear();
        // Drop mutexes.
        process_inner.mutex_list.clear();
        // Drop semaphores.
        process_inner.semaphore_list.clear();
        // Drop condvars.
        process_inner.condvar_list.clear();
        // Remove all threads, except for the main thread. Deallocate the kernel stacks of these threads.
        // We are still using the kernel stack of the main thread, so the TCB of the main thread must not be deallocated.
        // The TCB (including the kernel stack) of the main thread will be deallocated when the processs is reaped via waitpid.
        // There is no need to deallocate the tids, because the process itself is dead.
        while process_inner.tasks.len() > 1 {
            process_inner.tasks.pop();
        }
    }
    // process must be dropped manually, because schedule never returns.
    drop(process);
    let mut unused_task_cx = TaskContext::zero_init();
    schedule(&mut unused_task_cx as *mut _);
}

pub fn check_signals_of_current() -> Option<(i32, &'static str)> {
    current_process()
        .inner_exclusive_access()
        .signals
        .check_error()
}

pub fn current_add_signal(signal: SignalFlags) {
    current_process().inner_exclusive_access().signals |= signal;
}

/// Remove all Arc references pointing to *task, except those that belong to the corresponding PCB.
pub fn remove_inactive_task(task: Arc<TaskControlBlock>) {
    remove_task(task.clone());
    remove_timer(task.clone());
}

// fn call_kernel_signal_handler(signal: SignalFlags) {
//     let task = current_task().unwrap();
//     let mut inner = task.inner_exclusive_access();
//     match signal {
//         SignalFlags::SIGSTOP => {
//             if inner.signals.contains(SignalFlags::SIGSTOP) {
//                 inner.frozen = true;
//                 inner.signals ^= SignalFlags::SIGSTOP;
//             }
//         }
//         SignalFlags::SIGCONT => {
//             if inner.signals.contains(SignalFlags::SIGCONT) {
//                 inner.frozen = false;
//                 inner.signals ^= SignalFlags::SIGCONT;
//             }
//         }
//         _ => {
//             inner.killed = true;
//         }
//     }
// }

// fn call_user_signal_handler(signum: usize, signal: SignalFlags) {
//     let task = current_task().unwrap();
//     let mut inner = task.inner_exclusive_access();
//     let handler = inner.signal_actions.table[signum].handler;
//     if handler != 0 {
//         inner.handling_signal = signum as isize;
//         inner.signals ^= signal;
//         let trap_ctx = inner.get_trap_cx();
//         // copy the original trap_ctx to trap_ctx_backup
//         inner.trap_ctx_backup = Some(*trap_ctx);
//         trap_ctx.sepc = handler;
//         trap_ctx.gprs[10] = signum;
//     } else {
//         // default action
//         println!(
//             "[kernel] task/call_user_signal_handler: default action: ignore it or kill process"
//         );
//     }
// }

// fn check_pending_signals() {
//     for signum in 0..=MAX_SIG {
//         let task = current_task().unwrap();
//         let inner = task.inner_exclusive_access();
//         let signal = SignalFlags::from_bits(1 << signum).unwrap();
//         if inner.signals.contains(signal) && !inner.signal_mask.contains(signal) {
//             let handling_signal = inner.handling_signal;
//             if handling_signal == -1
//                 || !inner.signal_actions.table[handling_signal as usize]
//                     .mask
//                     .contains(signal)
//             {
//                 drop(inner);
//                 drop(task);
//                 if signal == SignalFlags::SIGKILL
//                     || signal == SignalFlags::SIGSTOP
//                     || signal == SignalFlags::SIGCONT
//                     || signal == SignalFlags::SIGDEF
//                 {
//                     call_kernel_signal_handler(signal);
//                 } else {
//                     call_user_signal_handler(signum, signal);
//                     return;
//                 }
//             }
//         }
//     }
// }

// pub fn handle_signals() {
//     loop {
//         check_pending_signals();
//         let (frozen, killed) = {
//             let task = current_task().unwrap();
//             let inner = task.inner_exclusive_access();
//             (inner.frozen, inner.killed)
//         };
//         if killed || !frozen {
//             break;
//         }
//         suspend_current_and_run_next();
//     }
// }
