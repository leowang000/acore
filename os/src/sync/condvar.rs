use crate::{
    sync::{Mutex, UPSafeCell},
    task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock},
};
use alloc::{collections::vec_deque::VecDeque, sync::Arc};

pub struct Condvar {
    waiter_queue: UPSafeCell<VecDeque<Arc<TaskControlBlock>>>,
}

impl Condvar {
    pub fn new() -> Self {
        Self {
            waiter_queue: UPSafeCell::new(VecDeque::new()),
        }
    }

    pub fn signal(&self) {
        let mut waiter_queue = self.waiter_queue.exclusive_access();
        if let Some(task) = waiter_queue.pop_front() {
            wakeup_task(task);
        }
    }

    pub fn wait(&self, mutex: Arc<dyn Mutex>) {
        mutex.unlock();
        self.waiter_queue
            .exclusive_access()
            .push_back(current_task());
        block_current_and_run_next();
        mutex.lock();
    }
}
