use alloc::{collections::vec_deque::VecDeque, sync::Arc};

use crate::{
    sync::UPSafeCell,
    task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock},
};

pub struct Semaphore {
    inner: UPSafeCell<SemaphoreInner>,
}

struct SemaphoreInner {
    count: isize,
    waiter_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    pub fn new(res_count: usize) -> Self {
        Self {
            inner: UPSafeCell::new(SemaphoreInner {
                count: res_count as isize,
                waiter_queue: VecDeque::new(),
            }),
        }
    }

    pub fn up(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        if inner.count <= 0 {
            if let Some(task) = inner.waiter_queue.pop_front() {
                wakeup_task(task);
            }
        }
    }

    pub fn down(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.waiter_queue.push_back(current_task());
            drop(inner);
            block_current_and_run_next();
        }
    }
}
