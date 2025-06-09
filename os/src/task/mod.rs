use crate::{
    fs::{open_file, OpenFlags},
    println,
    task::manager::remove_from_pid2task,
};
use alloc::sync::Arc;
use lazy_static::lazy_static;
use processor::take_current_task;
use task::*;

mod action;
mod context;
mod kernel_stack;
mod manager;
mod pid;
mod processor;
mod signal;
mod switch;
mod task;

pub use action::{SignalAction, SignalActionTable};
pub use context::TaskContext;
pub use kernel_stack::KernelStack;
pub use manager::{add_task, pid2task};
pub use pid::{pid_alloc, PidAllocator, PidHandle};
pub use processor::{
    current_task, current_task_satp, current_task_trap_cx, run_tasks, schedule, Processor,
};
pub use signal::{SignalFlags, MAX_SIG};

lazy_static! {
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new({
        let inode = open_file("initproc", OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        TaskControlBlock::new(v.as_slice())
    });
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
    remove_from_pid2task(current_task.get_pid());
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

pub fn check_signals_error_of_current() -> Option<(i32, &'static str)> {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .signals
        .check_error()
}

pub fn current_add_signal(signal: SignalFlags) {
    current_task().unwrap().inner_exclusive_access().signals |= signal;
}

fn call_kernel_signal_handler(signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    match signal {
        SignalFlags::SIGSTOP => {
            if inner.signals.contains(SignalFlags::SIGSTOP) {
                inner.frozen = true;
                inner.signals ^= SignalFlags::SIGSTOP;
            }
        }
        SignalFlags::SIGCONT => {
            if inner.signals.contains(SignalFlags::SIGCONT) {
                inner.frozen = false;
                inner.signals ^= SignalFlags::SIGCONT;
            }
        }
        _ => {
            inner.killed = true;
        }
    }
}

fn call_user_signal_handler(signum: usize, signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let handler = inner.signal_actions.table[signum].handler;
    if handler != 0 {
        inner.handling_signal = signum as isize;
        inner.signals ^= signal;
        let trap_ctx = inner.get_trap_cx();
        // copy the original trap_ctx to trap_ctx_backup
        inner.trap_ctx_backup = Some(*trap_ctx);
        trap_ctx.sepc = handler;
        trap_ctx.gprs[10] = signum;
    } else {
        // default action
        println!(
            "[kernel] task/call_user_signal_handler: default action: ignore it or kill process"
        );
    }
}

fn check_pending_signals() {
    for signum in 0..=MAX_SIG {
        let task = current_task().unwrap();
        let inner = task.inner_exclusive_access();
        let signal = SignalFlags::from_bits(1 << signum).unwrap();
        if inner.signals.contains(signal) && !inner.signal_mask.contains(signal) {
            let handling_signal = inner.handling_signal;
            if handling_signal == -1
                || !inner.signal_actions.table[handling_signal as usize]
                    .mask
                    .contains(signal)
            {
                drop(inner);
                drop(task);
                if signal == SignalFlags::SIGKILL
                    || signal == SignalFlags::SIGSTOP
                    || signal == SignalFlags::SIGCONT
                    || signal == SignalFlags::SIGDEF
                {
                    call_kernel_signal_handler(signal);
                } else {
                    call_user_signal_handler(signum, signal);
                    return;
                }
            }
        }
    }
}

pub fn handle_signals() {
    loop {
        check_pending_signals();
        let (frozen, killed) = {
            let task = current_task().unwrap();
            let inner = task.inner_exclusive_access();
            (inner.frozen, inner.killed)
        };
        if killed || !frozen {
            break;
        }
        suspend_current_and_run_next();
    }
}
