use crate::{
    sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore},
    task::{block_current_and_run_next, current_process, current_task},
    timer::{add_timer, get_time_ms},
};
use alloc::sync::Arc;

pub fn sys_sleep(sleep_ms: usize) -> isize {
    let expire_ms = get_time_ms() + sleep_ms;
    add_timer(expire_ms, current_task());
    block_current_and_run_next();
    0
}

pub fn sys_mutex_create(blocking: bool) -> isize {
    let process = current_process();
    let mutex: Arc<dyn Mutex> = if blocking {
        Arc::new(MutexBlocking::new())
    } else {
        Arc::new(MutexSpin::new())
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some((id, _)) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, m)| m.is_none())
    {
        process_inner.mutex_list[id] = Some(mutex);
        id as isize
    } else {
        process_inner.mutex_list.push(Some(mutex));
        process_inner.mutex_list.len() as isize - 1
    }
}

pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = process_inner.mutex_list[mutex_id].as_ref().unwrap().clone();
    drop(process_inner);
    drop(process);
    mutex.lock();
    0
}

pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = process_inner.mutex_list[mutex_id].as_ref().unwrap().clone();
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}

pub fn sys_semaphore_create(res_count: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let semaphore = Arc::new(Semaphore::new(res_count));
    if let Some((id, _)) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, s)| s.is_none())
    {
        process_inner.semaphore_list[id] = Some(semaphore);
        id as isize
    } else {
        process_inner.semaphore_list.push(Some(semaphore));
        process_inner.semaphore_list.len() as isize - 1
    }
}

pub fn sys_semaphore_up(sem_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let semaphore = process_inner.semaphore_list[sem_id]
        .as_ref()
        .unwrap()
        .clone();
    drop(process_inner);
    semaphore.up();
    0
}

pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let semaphore = process_inner.semaphore_list[sem_id]
        .as_ref()
        .unwrap()
        .clone();
    drop(process_inner);
    semaphore.down();
    0
}

pub fn sys_condvar_create() -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let condvar = Arc::new(Condvar::new());
    if let Some((id, _)) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, s)| s.is_none())
    {
        process_inner.condvar_list[id] = Some(condvar);
        id as isize
    } else {
        process_inner.condvar_list.push(Some(condvar));
        process_inner.condvar_list.len() as isize - 1
    }
}

pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = process_inner.condvar_list[condvar_id]
        .as_ref()
        .unwrap()
        .clone();
    drop(process_inner);
    condvar.signal();
    0
}

pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = process_inner.condvar_list[condvar_id]
        .as_ref()
        .unwrap()
        .clone();
    let mutex = process_inner.mutex_list[mutex_id].as_ref().unwrap().clone();
    drop(process_inner);
    condvar.wait(mutex);
    0
}
