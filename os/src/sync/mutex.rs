use alloc::{collections::vec_deque::VecDeque, sync::Arc};

use crate::{
    sync::UPSafeCell,
    task::{
        block_current_and_run_next, current_task, suspend_current_and_run_next, wakeup_task,
        TaskControlBlock,
    },
};

pub trait Mutex: Sync + Send {
    fn lock(&self);
    fn unlock(&self);
}

pub struct MutexSpin {
    locked: UPSafeCell<bool>,
}

impl MutexSpin {
    pub fn new() -> Self {
        Self {
            locked: UPSafeCell::new(false),
        }
    }
}

impl Mutex for MutexSpin {
    fn lock(&self) {
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                *locked = true;
                return;
            }
        }
    }

    fn unlock(&self) {
        let mut locked = self.locked.exclusive_access();
        *locked = false;
    }
}

pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

struct MutexBlockingInner {
    locked: bool,
    waiter_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    pub fn new() -> Self {
        Self {
            inner: UPSafeCell::new(MutexBlockingInner {
                locked: false,
                waiter_queue: VecDeque::new(),
            }),
        }
    }
}

impl Mutex for MutexBlocking {
    fn lock(&self) {
        let mut inner = self.inner.exclusive_access();
        if inner.locked {
            inner.waiter_queue.push_back(current_task());
            drop(inner);
            block_current_and_run_next();
        } else {
            inner.locked = true;
        }
    }

    fn unlock(&self) {
        let mut inner = self.inner.exclusive_access();
        assert!(inner.locked);
        if let Some(waiter) = inner.waiter_queue.pop_front() {
            wakeup_task(waiter);
        } else {
            inner.locked = false;
        }
    }
}
