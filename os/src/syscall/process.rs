use alloc::sync::Arc;

use crate::{
    loader::get_app_data_by_name,
    mm::{translated_refmut, traslated_str},
    println,
    task::{
        add_task, current_task, current_task_satp, exit_current_and_run_next,
        suspend_current_and_run_next,
    },
    timer::get_time_ms,
};

pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next(exit_code);
    unreachable!();
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

pub fn sys_getpid() -> isize {
    current_task().unwrap().get_pid() as isize
}

pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    new_task.inner_exclusive_access().get_trap_cx().gprs[10] = 0;
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    let satp = current_task_satp();
    let path = traslated_str(satp, path);
    if let Some(data) = get_app_data_by_name(&path) {
        current_task().unwrap().exec(data);
        0
    } else {
        -1
    }
}

pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let current_task = current_task().unwrap();
    let mut current_task_inner = current_task.inner_exclusive_access();
    if !current_task_inner
        .children
        .iter()
        .any(|task_control_block| pid == -1 || pid as usize == task_control_block.get_pid())
    {
        return -1;
    }
    if let Some((id, _)) =
        current_task_inner
            .children
            .iter()
            .enumerate()
            .find(|(_, task_control_block)| {
                task_control_block.inner_exclusive_access().is_zombie()
                    && (pid == -1 || pid as usize == task_control_block.get_pid())
            })
    {
        let child = current_task_inner.children.remove(id);
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.get_pid();
        let child_exit_code = child.inner_exclusive_access().exit_code;
        *translated_refmut(current_task_inner.satp(), exit_code_ptr) = child_exit_code;
        found_pid as isize
    } else {
        -2
    }
}
