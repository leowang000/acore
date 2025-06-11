use alloc::{string::String, sync::Arc, vec::Vec};

use crate::{
    fs::{open_file, OpenFlags},
    mm::{translated_ref, translated_refmut, translated_str},
    task::{
        current_process, current_task_satp, exit_current_and_run_next, pid2process,
        suspend_current_and_run_next, SignalFlags,
    },
    timer::get_time_ms,
};

pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
    unreachable!();
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_kill(pid: usize, signal: u32) -> isize {
    if let Some(process) = pid2process(pid) {
        if let Some(flag) = SignalFlags::from_bits(signal) {
            let mut inner = process.inner_exclusive_access();
            if inner.signals.contains(flag) {
                return -1;
            }
            inner.signals |= flag;
            0
        } else {
            -1
        }
    } else {
        -1
    }
}

// fn check_sigaction_error(flag: SignalFlags, action: usize, old_action: usize) -> bool {
//     action == 0 || old_action == 0 || flag == SignalFlags::SIGKILL || flag == SignalFlags::SIGSTOP
// }

// pub fn sys_sigaction(
//     signum: i32,
//     action: *const SignalAction,
//     old_action: *mut SignalAction,
// ) -> isize {
//     let satp = current_task_satp();
//     let task = current_task().unwrap();
//     let mut inner = task.inner_exclusive_access();
//     if signum as usize > MAX_SIG {
//         return -1;
//     }
//     if let Some(flag) = SignalFlags::from_bits(1 << signum) {
//         if check_sigaction_error(flag, action as usize, old_action as usize) {
//             return -1;
//         }
//         *translated_refmut(satp, old_action) = inner.signal_actions.table[signum as usize];
//         inner.signal_actions.table[signum as usize] = *translated_ref(satp, action);
//         0
//     } else {
//         -1
//     }
// }

// pub fn sys_sigprocmask(mask: u32) -> isize {
//     let task = current_task().unwrap();
//     let mut inner = task.inner_exclusive_access();
//     let old_mask = inner.signal_mask;
//     if let Some(flag) = SignalFlags::from_bits(mask) {
//         inner.signal_mask = flag;
//         old_mask.bits() as isize
//     } else {
//         -1
//     }
// }

// pub fn sys_sigreturn() -> isize {
//     let task = current_task().unwrap();
//     let mut inner = task.inner_exclusive_access();
//     inner.handling_signal = -1;
//     let trap_ctx = inner.get_trap_cx();
//     *trap_ctx = inner.trap_ctx_backup.unwrap();
//     trap_ctx.gprs[10] as isize
// }

pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

pub fn sys_getpid() -> isize {
    current_process().get_pid() as isize
}

pub fn sys_fork() -> isize {
    let current_process = current_process();
    let new_process = current_process.fork();
    let new_pid = new_process.get_pid();
    let new_process_inner = new_process.inner_exclusive_access();
    new_process_inner.tasks[0]
        .as_ref()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
        .gprs[10] = 0;
    new_pid as isize
}

#[no_mangle]
pub fn sys_exec(path: *const u8, mut args: *const usize) -> isize {
    let satp = current_task_satp();
    let path = translated_str(satp, path);
    let mut args_vec: Vec<String> = Vec::new();
    loop {
        let arg_str_ptr = *translated_ref(satp, args);
        if arg_str_ptr == 0 {
            break;
        }
        args_vec.push(translated_str(satp, arg_str_ptr as *const u8));
        unsafe { args = args.add(1) }
    }
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let process = current_process();
        let argc = args_vec.len();
        process.exec(all_data.as_slice(), args_vec);
        // a0 will be covered by the return value of sys_exec, so the first argument (argc) should be returned.
        argc as isize
    } else {
        -1
    }
}

pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|pcb| pid == -1 || pid as usize == pcb.get_pid())
    {
        return -1;
    }
    if let Some((id, _)) = inner.children.iter().enumerate().find(|(_, pcb)| {
        pcb.inner_exclusive_access().is_zombie && (pid == -1 || pid as usize == pcb.get_pid())
    }) {
        let child = inner.children.remove(id);
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.get_pid();
        let child_exit_code = child.inner_exclusive_access().exit_code;
        *translated_refmut(inner.address_space.satp(), exit_code_ptr) = child_exit_code;
        found_pid as isize
    } else {
        -2
    }
}
